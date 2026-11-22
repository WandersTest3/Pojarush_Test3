//! OptiFine Custom Entity Models: `assets/minecraft/optifine/cem/<entity>.jem` in a resource
//! pack replaces or extends a mob's model parts (`.jpm` part files included) and animates them
//! with expressions, as `cem_model.txt`, `cem_part.txt` and `cem_animation.txt` define.
//!
//! As OptiFine does, a replaced part keeps its vanilla pivot and animation and loses its own
//! cubes; the pack's model hangs under it as a child whose rotation point is the inverted
//! `translate`, and whose boxes are given in the inverted axes (`invertAxis`, usually `xy`).
use super::{Part, Polygon, anim::Poses};
use serde_json::Value;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, RwLock},
};

/// A mob type's custom model: its texture and its animations, the parts being in the model.
pub struct Cem {
    /// The atlas sprite of the jem's texture (`entity/...`), when it names one there.
    pub texture: Option<String>,
    pub animations: Vec<(Target, Expr)>,
    /// The `var.*` names the animations use, by index.
    vars: Vec<String>,
}

/// Each mob type's custom model and animations, with the address of the vanilla model they
/// replace: the server thread loads the mob types again without the resource packs, and a pack
/// reload swaps them without reloading the mob types.
type Entry = (usize, Arc<Part>, Arc<Cem>);
static TYPES: RwLock<Option<HashMap<String, Entry>>> = RwLock::new(None);
static VARS: Mutex<Option<HashMap<u32, Vec<f32>>>> = Mutex::new(None);

/// The custom model of mob type `id` (`minecraft:creeper`) whose vanilla model is `model`, if a
/// pack has one: the model to draw and its animations.
pub fn get(id: &str, model: &Arc<Part>) -> Option<(Arc<Part>, Arc<Cem>)> {
    let t = TYPES.read().ok()?;
    let (at, custom, cem) = t.as_ref()?.get(id)?;
    (*at == Arc::as_ptr(model) as usize).then(|| (custom.clone(), cem.clone()))
}

/// Applies `<root>/assets/minecraft/optifine/cem/<name>.jem` to `model`, registering the new
/// model and its animations for it; true when the pack has one for `id`.
pub fn apply(root: &Path, id: &str, model: &Arc<Part>) -> bool {
    let name = id.rsplit(':').next().unwrap_or(id);
    let dir = root.join("assets/minecraft/optifine/cem");
    let path = dir.join(format!("{name}.jem"));
    let Some(jem) = std::fs::read(&path).ok().and_then(|t| serde_json::from_slice::<Value>(&t).ok()) else {
        return false;
    };
    let mut custom = Part::clone(model);
    let (cem, ok) = build(root, &dir, &jem, &mut custom);
    if !ok {
        return false;
    }
    if let Ok(mut t) = TYPES.write() {
        t.get_or_insert_with(HashMap::new)
            .insert(id.to_owned(), (Arc::as_ptr(model) as usize, Arc::new(custom), Arc::new(cem)));
    }
    if let Ok(mut v) = VARS.lock() {
        *v = None;
    }
    true
}

/// Forgets every custom model and applies the packs under `root` again to `types` (each mob
/// type's id and vanilla model): `Minecraft.reloadResourcePacks` for the entity models.
pub fn reload<'a>(root: &Path, types: impl IntoIterator<Item = (&'a String, &'a super::MobType)>) -> usize {
    if let Ok(mut t) = TYPES.write() {
        *t = None;
    }
    if let Ok(mut v) = VARS.lock() {
        *v = None;
    }
    types.into_iter().map(|(id, t)| apply_type(root, id, t)).sum()
}

/// Applies the packs to a mob type's model and to its model layers that have a jem of their own
/// (`zombie_baby.jem` for the `zombie_baby#main` layer); how many models a pack replaced.
pub fn apply_type(root: &Path, id: &str, t: &super::MobType) -> usize {
    let mut n = usize::from(apply(root, id, &t.model));
    for (key, model) in &t.layers {
        if let Some(layer) = layer_id(key) {
            n += usize::from(apply(root, &layer, model));
        }
    }
    n
}

/// The jem name of a model layer: `cow_baby#main` is `minecraft:cow_baby`, and another layer
/// is its model and layer joined, as OptiFine names them (`sheep#wool` is `sheep_wool`,
/// `sheep_baby#wool` is `sheep_baby_wool`).
pub fn layer_id(key: &str) -> Option<String> {
    let (name, layer) = key.split_once('#')?;
    if name.is_empty() || layer.is_empty() || layer.contains('#') {
        return None;
    }
    Some(if layer == "main" {
        format!("minecraft:{name}")
    } else {
        format!("minecraft:{name}_{layer}")
    })
}

fn floats<const N: usize>(v: &Value, default: [f32; N]) -> [f32; N] {
    let mut out = default;
    if let Some(a) = v.as_array() {
        for (i, x) in a.iter().take(N).enumerate() {
            out[i] = x.as_f64().unwrap_or(f64::from(default[i])) as f32;
        }
    }
    out
}

/// OptiFine's names for vanilla parts that Mojang names differently.
fn vanilla_name(part: &str) -> &str {
    match part {
        "headwear" => "hat",
        "headwear2" => "hat_rim",
        "bodywear" => "jacket",
        "leg1" => "right_hind_leg",
        "leg2" => "left_hind_leg",
        "leg3" => "right_front_leg",
        "leg4" => "left_front_leg",
        "bill" => "beak",
        "chin" => "red_thing",
        "jaw" => "jaw",
        other => other,
    }
}

fn names(part: &Part, out: &mut Vec<String>) {
    if !part.name.starts_with("cem#") {
        out.push(part.name.clone());
    }
    for c in &part.children {
        names(c, out);
    }
}

/// A part's name as a bag of words, so `left_hind_leg` and `hind_left_leg` compare equal.
fn bag(name: &str) -> String {
    let mut words: Vec<String> = name
        .to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect();
    words.sort();
    words.concat()
}

/// The vanilla part a jem part names: OptiFine's name for it, its own name, or one with the
/// same words in another order or case. Resource packs name parts after OptiFine's table for
/// each mob, which Mojang's names rarely match exactly.
fn resolve(model: &Part, part: &str) -> Option<String> {
    let vanilla = vanilla_name(part);
    if find(model, vanilla) {
        return Some(vanilla.to_owned());
    }
    if find(model, part) {
        return Some(part.to_owned());
    }
    let (want, alias) = (bag(part), bag(vanilla));
    let mut all = Vec::new();
    names(model, &mut all);
    all.into_iter().find(|n| {
        let b = bag(n);
        b == want || b == alias
    })
}

fn find_mut<'a>(part: &'a mut Part, name: &str) -> Option<&'a mut Part> {
    if part.name == name {
        return Some(part);
    }
    part.children.iter_mut().find_map(|c| find_mut(c, name))
}

fn find(part: &Part, name: &str) -> bool {
    part.name == name || part.children.iter().any(|c| find(c, name))
}

/// Resolves a jem texture path to an atlas sprite (`entity/...`), as cem_part.txt resolves it.
fn texture_sprite(dir: &Path, root: &Path, t: &str) -> Option<String> {
    let t = t.strip_suffix(".png").unwrap_or(t);
    let rel: PathBuf = if let Some(rest) = t.strip_prefix("./") {
        dir.join(rest)
    } else if let Some(rest) = t.strip_prefix("~/") {
        root.join("assets/minecraft/optifine").join(rest)
    } else if let Some((ns, rest)) = t.split_once(':') {
        root.join("assets").join(ns).join(rest)
    } else if t.contains('/') {
        root.join("assets/minecraft").join(t)
    } else {
        dir.join(t)
    };
    let rel = rel.strip_prefix(root.join("assets/minecraft/textures")).ok()?;
    let s = rel.to_string_lossy().replace('\\', "/");
    s.starts_with("entity/").then_some(s)
}

struct Builder<'a> {
    root: &'a Path,
    dir: &'a Path,
    count: usize,
    /// Custom part ids to their names in the model.
    ids: Vec<(String, String)>,
    tex: [f32; 2],
}

impl Builder<'_> {
    /// One jem/jpm part model: its own rotation point, rotation, boxes and submodels.
    fn part(&mut self, v: &Value, tex: [f32; 2]) -> Part {
        let v = &self.resolved(v);
        let tex = floats(&v["textureSize"], tex);
        let axes = v["invertAxis"].as_str().unwrap_or("");
        let inv = [axes.contains('x'), axes.contains('y'), axes.contains('z')];
        let mut t = floats(&v["translate"], [0.0; 3]);
        let mut r = floats(&v["rotate"], [0.0; 3]).map(f32::to_radians);
        for i in 0..3 {
            if inv[i] {
                t[i] = -t[i];
                r[i] = -r[i];
            }
        }
        let s = v["scale"].as_f64().unwrap_or(1.0) as f32;
        self.count += 1;
        let id = v["id"].as_str().unwrap_or("");
        let name = format!("cem#{}#{id}", self.count);
        if !id.is_empty() {
            self.ids.push((id.to_owned(), name.clone()));
        }
        let mirror = v["mirrorTexture"].as_str().unwrap_or("").contains('u');
        let mut polygons = Vec::new();
        for b in v["boxes"].as_array().into_iter().flatten() {
            let c = floats(&b["coordinates"], [0.0; 6]);
            let (mut x, mut y, mut z) = (c[0], c[1], c[2]);
            let (w, h, d) = (c[3], c[4], c[5]);
            if inv[0] {
                x = -x - w;
            }
            if inv[1] {
                y = -y - h;
            }
            if inv[2] {
                z = -z - d;
            }
            let add = b["sizeAdd"].as_f64().unwrap_or(0.0) as f32;
            let grow = floats(&b["sizesAdd"], [add; 3]);
            let faces = [
                ("uvDown", "uvDown"),
                ("uvUp", "uvUp"),
                ("uvWest", "uvLeft"),
                ("uvNorth", "uvFront"),
                ("uvEast", "uvRight"),
                ("uvSouth", "uvBack"),
            ]
            .map(|(a, alias)| {
                let f = if b[a].is_array() { &b[a] } else { &b[alias] };
                f.is_array().then(|| floats(f, [0.0; 4]))
            });
            let offset = floats(&b["textureOffset"], [0.0; 2]);
            polygons.extend(cube_polygons(offset, [x, y, z], [w, h, d], grow, mirror, tex, &faces));
        }
        let mut children = Vec::new();
        if v["submodel"].is_object() {
            children.push(self.part(&v["submodel"], tex));
        }
        for sub in v["submodels"].as_array().into_iter().flatten() {
            children.push(self.part(sub, tex));
        }
        Part {
            name,
            pose: [t[0], t[1], t[2], r[0], r[1], r[2], s, s, s],
            polygons,
            children,
        }
    }

    /// A part with `"model": "x.jpm"` merged: the jpm's fields under the entry's own.
    fn resolved(&self, v: &Value) -> Value {
        let Some(file) = v["model"].as_str() else { return v.clone() };
        let path = if file.contains('/') {
            self.root.join("assets/minecraft").join(file)
        } else {
            self.dir.join(file)
        };
        let Some(Value::Object(mut base)) = std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        else {
            return v.clone();
        };
        if let Value::Object(own) = v {
            for (k, x) in own {
                if k != "model" {
                    base.insert(k.clone(), x.clone());
                }
            }
        }
        Value::Object(base)
    }
}

/// `ModelPart.Cube`: six faces of a box in pixels, UV from `textureOffset` (box layout) or the
/// faces' own rectangles, scaled by the texture size.
pub fn cube_polygons(
    uv: [f32; 2],
    from: [f32; 3],
    size: [f32; 3],
    grow: [f32; 3],
    mirror: bool,
    tex: [f32; 2],
    faces: &[Option<[f32; 4]>; 6],
) -> Vec<Polygon> {
    let [w, h, d] = size;
    let (mut x0, y0, z0) = (from[0] - grow[0], from[1] - grow[1], from[2] - grow[2]);
    let (mut x1, y1, z1) = (from[0] + w + grow[0], from[1] + h + grow[1], from[2] + d + grow[2]);
    if mirror {
        std::mem::swap(&mut x0, &mut x1);
    }
    let p = [
        [x0, y0, z0],
        [x1, y0, z0],
        [x1, y1, z0],
        [x0, y1, z0],
        [x0, y0, z1],
        [x1, y0, z1],
        [x1, y1, z1],
        [x0, y1, z1],
    ];
    let (u, v) = (uv[0], uv[1]);
    let ua = u;
    let ub = u + d;
    let uc = u + d + w;
    let ud = u + d + w + w;
    let ue = u + d + w + d;
    let uf = u + d + w + d + w;
    let va = v;
    let vb = v + d;
    let vc = v + d + h;
    let boxed: [([usize; 4], [f32; 4], [f32; 3]); 6] = [
        ([5, 4, 0, 1], [ub, va, uc, vb], [0.0, -1.0, 0.0]),
        ([2, 3, 7, 6], [uc, vb, ud, va], [0.0, 1.0, 0.0]),
        ([0, 4, 7, 3], [ua, vb, ub, vc], [-1.0, 0.0, 0.0]),
        ([1, 0, 3, 2], [ub, vb, uc, vc], [0.0, 0.0, -1.0]),
        ([5, 1, 2, 6], [uc, vb, ue, vc], [1.0, 0.0, 0.0]),
        ([4, 5, 6, 7], [ue, vb, uf, vc], [0.0, 0.0, 1.0]),
    ];
    let mut out = Vec::with_capacity(6);
    for (i, (idx, rect, normal)) in boxed.iter().enumerate() {
        let explicit = faces.iter().any(Option::is_some);
        let [u1, v1, u2, v2] = match faces[i] {
            Some(r) => r,
            None if explicit => continue,
            None => *rect,
        };
        let corners = [[u2, v1], [u1, v1], [u1, v2], [u2, v2]];
        let mut vertices: [[f32; 5]; 4] = std::array::from_fn(|k| {
            let q = p[idx[k]];
            [q[0], q[1], q[2], corners[k][0] / tex[0], corners[k][1] / tex[1]]
        });
        let mut normal = *normal;
        if mirror {
            vertices.reverse();
            normal[0] = -normal[0];
        }
        out.push(Polygon { normal, vertices });
    }
    out
}

/// Builds the model and animations of a jem; false when nothing in it applies to `model`.
fn build(root: &Path, dir: &Path, jem: &Value, model: &mut Part) -> (Cem, bool) {
    let tex = floats(&jem["textureSize"], [64.0, 32.0]);
    let mut b = Builder { root, dir, count: 0, ids: Vec::new(), tex };
    let mut applied = false;
    let mut blocks: Vec<(String, String, Value)> = Vec::new();
    for entry in jem["models"].as_array().into_iter().flatten() {
        let Some(part) = entry["part"].as_str() else { continue };
        let Some(target) = resolve(model, part) else {
            log::debug!("Horizon: CEM part `{part}` matches no part of the model");
            continue;
        };
        let target = target.as_str();
        let attach = entry["attach"].as_bool().unwrap_or(false);
        let custom = b.part(entry, b.tex);
        let custom_name = custom.name.clone();
        if let Some(p) = find_mut(model, target) {
            if !attach {
                p.polygons.clear();
            }
            p.children.push(custom);
            applied = true;
        }
        if let Some(Value::Array(anims)) = b.resolved(entry).get("animations") {
            for a in anims {
                blocks.push((custom_name.clone(), target.to_owned(), a.clone()));
            }
        }
    }
    let texture = jem["texture"].as_str().and_then(|t| texture_sprite(dir, root, t));
    let mut animations = Vec::new();
    let mut vars = Vec::new();
    for (this, part, block) in blocks {
        let Value::Object(map) = block else { continue };
        let mut scope = Scope { this: &this, part: &part, model, ids: &b.ids, vars: &mut vars };
        for (k, e) in map {
            let Some(text) = e.as_str().map(str::to_owned).or_else(|| e.as_f64().map(|f| f.to_string())) else {
                continue;
            };
            let (Some(target), Ok(expr)) = (scope.target(&k), parse(&text, &mut scope)) else { continue };
            animations.push((target, expr));
        }
    }
    (Cem { texture, animations, vars }, applied)
}

struct Scope<'a> {
    this: &'a str,
    part: &'a str,
    model: &'a Part,
    ids: &'a [(String, String)],
    vars: &'a mut Vec<String>,
}

impl Scope<'_> {
    fn model_name(&self, spec: &str) -> Option<String> {
        let mut segs = spec.split(':');
        let first = segs.next()?;
        let mut name = match first {
            "this" => self.this.to_owned(),
            "part" => self.part.to_owned(),
            other => match resolve(self.model, other) {
                Some(v) => v,
                None => self.ids.iter().find(|(id, _)| id == other)?.1.clone(),
            },
        };
        for seg in segs {
            let base = sub(self.model, &name)?;
            name = deep_id(base, seg)?;
        }
        Some(name)
    }

    fn var(&mut self, name: &str) -> usize {
        if let Some(i) = self.vars.iter().position(|v| v == name) {
            return i;
        }
        self.vars.push(name.to_owned());
        self.vars.len() - 1
    }

    fn target(&mut self, key: &str) -> Option<Target> {
        if let Some(v) = key.strip_prefix("var.").or_else(|| key.strip_prefix("varb.")) {
            return Some(Target::Var(self.var(v)));
        }
        let (model, field) = key.rsplit_once('.')?;
        let field = Field::parse(field)?;
        Some(Target::Part(self.model_name(model)?, field))
    }
}

fn sub<'a>(p: &'a Part, name: &str) -> Option<&'a Part> {
    if p.name == name {
        return Some(p);
    }
    p.children.iter().find_map(|c| sub(c, name))
}

fn deep_id(p: &Part, id: &str) -> Option<String> {
    for c in &p.children {
        if c.name.rsplit('#').next() == Some(id) {
            return Some(c.name.clone());
        }
        if let Some(n) = deep_id(c, id) {
            return Some(n);
        }
    }
    None
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Field {
    Pose(usize),
    Visible,
    VisibleBoxes,
}
impl Field {
    fn parse(f: &str) -> Option<Self> {
        Some(match f {
            "tx" => Self::Pose(0),
            "ty" => Self::Pose(1),
            "tz" => Self::Pose(2),
            "rx" => Self::Pose(3),
            "ry" => Self::Pose(4),
            "rz" => Self::Pose(5),
            "sx" => Self::Pose(6),
            "sy" => Self::Pose(7),
            "sz" => Self::Pose(8),
            "visible" => Self::Visible,
            "visible_boxes" => Self::VisibleBoxes,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    Part(String, Field),
    Var(usize),
}

/// What the expressions read about the mob being drawn.
#[derive(Clone, Copy, Debug, Default)]
pub enum Param {
    #[default]
    Zero,
    LimbSwing,
    LimbSpeed,
    Age,
    HeadPitch,
    HeadYaw,
    Health,
    MaxHealth,
    HurtTime,
    DeathTime,
    SwingProgress,
    PosX,
    PosY,
    PosZ,
    RotX,
    RotY,
    Id,
    FrameTime,
    Time,
    IsChild,
    IsOnGround,
    IsAlive,
    IsHurt,
    IsInWater,
    IsInvisible,
    IsBurning,
    IsRiding,
    IsRidden,
    IsSitting,
    IsTamed,
    IsAggressive,
    IsInLava,
    IsOnShoulder,
}

#[derive(Clone, Debug)]
pub enum Expr {
    Num(f32),
    Param(Param),
    Read(String, Field),
    Var(usize),
    Neg(Box<Expr>),
    Not(Box<Expr>),
    Bin(Op, Box<Expr>, Box<Expr>),
    Call(Func, Vec<Expr>),
}

#[derive(Clone, Copy, Debug)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    And,
    Or,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
}

#[derive(Clone, Copy, Debug)]
pub enum Func {
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Atan2,
    Torad,
    Todeg,
    Min,
    Max,
    Clamp,
    Abs,
    Floor,
    Ceil,
    Exp,
    Frac,
    Log,
    Pow,
    Random,
    Round,
    Signum,
    Sqrt,
    Fmod,
    Lerp,
    If,
    Between,
    Equals,
    In,
    Print,
}

fn func(name: &str) -> Option<Func> {
    use Func::*;
    Some(match name {
        "sin" => Sin,
        "cos" => Cos,
        "tan" => Tan,
        "asin" => Asin,
        "acos" => Acos,
        "atan" => Atan,
        "atan2" => Atan2,
        "torad" => Torad,
        "todeg" => Todeg,
        "min" => Min,
        "max" => Max,
        "clamp" => Clamp,
        "abs" => Abs,
        "floor" => Floor,
        "ceil" => Ceil,
        "exp" => Exp,
        "frac" => Frac,
        "log" => Log,
        "pow" => Pow,
        "random" => Random,
        "round" => Round,
        "signum" => Signum,
        "sqrt" => Sqrt,
        "fmod" => Fmod,
        "lerp" => Lerp,
        "if" | "ifb" => If,
        "between" => Between,
        "equals" => Equals,
        "in" => In,
        "print" | "printb" => Print,
        _ => return None,
    })
}

fn param(name: &str) -> Option<Param> {
    use Param::*;
    Some(match name {
        "limb_swing" => LimbSwing,
        "limb_speed" => LimbSpeed,
        "age" => Age,
        "head_pitch" => HeadPitch,
        "head_yaw" => HeadYaw,
        "health" => Health,
        "max_health" => MaxHealth,
        "hurt_time" => HurtTime,
        "death_time" => DeathTime,
        "swing_progress" => SwingProgress,
        "pos_x" | "player_pos_x" => PosX,
        "pos_y" | "player_pos_y" => PosY,
        "pos_z" | "player_pos_z" => PosZ,
        "rot_x" | "player_rot_x" => RotX,
        "rot_y" | "player_rot_y" => RotY,
        "id" => Id,
        "frame_time" => FrameTime,
        "time" | "frame_counter" => Time,
        "is_child" => IsChild,
        "is_on_ground" => IsOnGround,
        "is_alive" => IsAlive,
        "is_hurt" => IsHurt,
        "is_in_water" | "is_wet" => IsInWater,
        "is_invisible" => IsInvisible,
        "is_burning" => IsBurning,
        "is_riding" => IsRiding,
        "is_ridden" => IsRidden,
        "is_sitting" => IsSitting,
        "is_tamed" => IsTamed,
        "is_aggressive" => IsAggressive,
        "is_in_lava" => IsInLava,
        "is_on_shoulder" => IsOnShoulder,
        _ => Zero,
    })
}

struct Parser<'a, 'b> {
    s: &'a [u8],
    i: usize,
    scope: &'a mut Scope<'b>,
}

/// Parses an animation expression; names the scope cannot resolve read as 0, as OptiFine's
/// unknown parameters do.
fn parse(text: &str, scope: &mut Scope<'_>) -> Result<Expr, String> {
    let mut p = Parser { s: text.as_bytes(), i: 0, scope };
    let e = p.or()?;
    p.ws();
    if p.i < p.s.len() {
        return Err(format!("unexpected {:?} at {}", p.s[p.i] as char, p.i));
    }
    Ok(e)
}

impl Parser<'_, '_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }
    fn eat(&mut self, t: &str) -> bool {
        self.ws();
        if self.s[self.i..].starts_with(t.as_bytes()) {
            self.i += t.len();
            true
        } else {
            false
        }
    }
    fn bin(&mut self, ops: &[(&str, Op)], next: fn(&mut Self) -> Result<Expr, String>) -> Result<Expr, String> {
        let mut l = next(self)?;
        'outer: loop {
            for (t, op) in ops {
                if self.eat(t) {
                    let r = next(self)?;
                    l = Expr::Bin(*op, Box::new(l), Box::new(r));
                    continue 'outer;
                }
            }
            return Ok(l);
        }
    }
    fn or(&mut self) -> Result<Expr, String> {
        self.bin(&[("||", Op::Or)], Self::and)
    }
    fn and(&mut self) -> Result<Expr, String> {
        self.bin(&[("&&", Op::And)], Self::eq)
    }
    fn eq(&mut self) -> Result<Expr, String> {
        self.bin(&[("==", Op::Eq), ("!=", Op::Ne)], Self::cmp)
    }
    fn cmp(&mut self) -> Result<Expr, String> {
        self.bin(&[("<=", Op::Le), (">=", Op::Ge), ("<", Op::Lt), (">", Op::Gt)], Self::add)
    }
    fn add(&mut self) -> Result<Expr, String> {
        self.bin(&[("+", Op::Add), ("-", Op::Sub)], Self::mul)
    }
    fn mul(&mut self) -> Result<Expr, String> {
        self.bin(&[("*", Op::Mul), ("/", Op::Div), ("%", Op::Rem)], Self::unary)
    }
    fn unary(&mut self) -> Result<Expr, String> {
        if self.eat("!") {
            return Ok(Expr::Not(Box::new(self.unary()?)));
        }
        if self.eat("-") {
            return Ok(Expr::Neg(Box::new(self.unary()?)));
        }
        if self.eat("+") {
            return self.unary();
        }
        self.primary()
    }
    fn primary(&mut self) -> Result<Expr, String> {
        self.ws();
        if self.eat("(") {
            let e = self.or()?;
            if !self.eat(")") {
                return Err("missing )".into());
            }
            return Ok(e);
        }
        let start = self.i;
        let c = *self.s.get(self.i).ok_or("unexpected end")?;
        if c.is_ascii_digit() || c == b'.' {
            while self.i < self.s.len() && (self.s[self.i].is_ascii_digit() || self.s[self.i] == b'.') {
                self.i += 1;
            }
            let t = std::str::from_utf8(&self.s[start..self.i]).unwrap_or("0");
            return Ok(Expr::Num(t.parse().unwrap_or(0.0)));
        }
        while self.i < self.s.len()
            && (self.s[self.i].is_ascii_alphanumeric() || matches!(self.s[self.i], b'_' | b'.' | b':' | b'[' | b']'))
        {
            self.i += 1;
        }
        if self.i == start {
            return Err(format!("unexpected {:?}", c as char));
        }
        let name = std::str::from_utf8(&self.s[start..self.i]).unwrap_or("").to_owned();
        if self.eat("(") {
            let f = func(&name).ok_or_else(|| format!("unknown function {name}"))?;
            let mut args = Vec::new();
            if !self.eat(")") {
                loop {
                    args.push(self.or()?);
                    if self.eat(")") {
                        break;
                    }
                    if !self.eat(",") {
                        return Err("expected , or )".into());
                    }
                }
            }
            return Ok(Expr::Call(f, args));
        }
        Ok(match name.as_str() {
            "pi" => Expr::Num(std::f32::consts::PI),
            "true" => Expr::Num(1.0),
            "false" => Expr::Num(0.0),
            _ => {
                if let Some(v) = name.strip_prefix("var.").or_else(|| name.strip_prefix("varb.")) {
                    Expr::Var(self.scope.var(v))
                } else if let Some((model, field)) = name.rsplit_once('.')
                    && let Some(field) = Field::parse(field)
                {
                    match self.scope.model_name(model) {
                        Some(part) => Expr::Read(part, field),
                        None => Expr::Num(0.0),
                    }
                } else {
                    Expr::Param(param(&name).unwrap_or_default())
                }
            }
        })
    }
}

/// The values of the parameters for one mob in one frame.
pub struct Inputs {
    pub values: [f32; 40],
    pub id: u32,
}

fn truth(x: f32) -> bool {
    x != 0.0
}

impl Cem {
    /// Runs the animations on `poses` after vanilla's, in the jem's order.
    pub fn animate(&self, inputs: &Inputs, poses: &mut Poses) {
        if self.animations.is_empty() {
            return;
        }
        let mut vars = if self.vars.is_empty() {
            Vec::new()
        } else {
            VARS.lock()
                .ok()
                .and_then(|mut v| v.get_or_insert_with(HashMap::new).get(&inputs.id).cloned())
                .unwrap_or_default()
        };
        vars.resize(self.vars.len(), 0.0);
        for (target, expr) in &self.animations {
            let x = eval(expr, inputs, poses, &vars);
            if !x.is_finite() {
                continue;
            }
            match target {
                Target::Var(i) => vars[*i] = x,
                Target::Part(name, Field::Pose(i)) => poses.get(name)[*i] = x,
                Target::Part(name, Field::Visible) => poses.visible(name, truth(x)),
                Target::Part(name, Field::VisibleBoxes) => {
                    if !truth(x) {
                        poses.skip_draw(name);
                    }
                }
            }
        }
        if !self.vars.is_empty()
            && let Ok(mut v) = VARS.lock()
        {
            let map = v.get_or_insert_with(HashMap::new);
            if map.len() > 4096 {
                map.clear();
            }
            map.insert(inputs.id, vars);
        }
    }
}

fn eval(e: &Expr, inp: &Inputs, poses: &mut Poses, vars: &[f32]) -> f32 {
    let b = |x: bool| if x { 1.0 } else { 0.0 };
    match e {
        Expr::Num(n) => *n,
        Expr::Param(p) => inp.values[*p as usize],
        Expr::Var(i) => vars.get(*i).copied().unwrap_or(0.0),
        Expr::Read(name, Field::Pose(i)) => poses.get(name)[*i],
        Expr::Read(name, Field::Visible | Field::VisibleBoxes) => b(!poses.hidden(name)),
        Expr::Neg(x) => -eval(x, inp, poses, vars),
        Expr::Not(x) => b(!truth(eval(x, inp, poses, vars))),
        Expr::Bin(op, l, r) => {
            let l = eval(l, inp, poses, vars);
            match op {
                Op::And if !truth(l) => return 0.0,
                Op::Or if truth(l) => return 1.0,
                _ => {}
            }
            let r = eval(r, inp, poses, vars);
            match op {
                Op::Add => l + r,
                Op::Sub => l - r,
                Op::Mul => l * r,
                Op::Div => l / r,
                Op::Rem => l % r,
                Op::And | Op::Or => b(truth(r)),
                Op::Lt => b(l < r),
                Op::Le => b(l <= r),
                Op::Gt => b(l > r),
                Op::Ge => b(l >= r),
                Op::Eq => b(l == r),
                Op::Ne => b(l != r),
            }
        }
        Expr::Call(Func::If, a) => {
            let mut i = 0;
            while i + 1 < a.len() {
                if truth(eval(&a[i], inp, poses, vars)) {
                    return eval(&a[i + 1], inp, poses, vars);
                }
                i += 2;
            }
            a.get(i).map_or(0.0, |x| eval(x, inp, poses, vars))
        }
        Expr::Call(f, a) => {
            let v: Vec<f32> = a.iter().map(|x| eval(x, inp, poses, vars)).collect();
            let g = |i: usize| v.get(i).copied().unwrap_or(0.0);
            match f {
                Func::Sin => g(0).sin(),
                Func::Cos => g(0).cos(),
                Func::Tan => g(0).tan(),
                Func::Asin => g(0).asin(),
                Func::Acos => g(0).acos(),
                Func::Atan => g(0).atan(),
                Func::Atan2 => g(0).atan2(g(1)),
                Func::Torad => g(0).to_radians(),
                Func::Todeg => g(0).to_degrees(),
                Func::Min => v.iter().copied().fold(f32::INFINITY, f32::min),
                Func::Max => v.iter().copied().fold(f32::NEG_INFINITY, f32::max),
                Func::Clamp => g(0).clamp(g(1).min(g(2)), g(2).max(g(1))),
                Func::Abs => g(0).abs(),
                Func::Floor => g(0).floor(),
                Func::Ceil => g(0).ceil(),
                Func::Exp => g(0).exp(),
                Func::Frac => g(0).fract(),
                Func::Log => g(0).ln(),
                Func::Pow => g(0).powf(g(1)),
                Func::Random => {
                    let seed = if v.is_empty() { inp.values[Param::Time as usize] } else { g(0) };
                    let x = (seed * 12.989_8 + inp.id as f32 * 78.233).sin() * 43_758.547;
                    x - x.floor()
                }
                Func::Round => g(0).round(),
                Func::Signum => g(0).signum() * b(g(0) != 0.0),
                Func::Sqrt => g(0).sqrt(),
                Func::Fmod => g(0) - g(1) * (g(0) / g(1)).floor(),
                Func::Lerp => g(1) + (g(2) - g(1)) * g(0),
                Func::Between => b(g(0) >= g(1) && g(0) <= g(2)),
                Func::Equals => b((g(0) - g(1)).abs() <= g(2)),
                Func::In => b(v.iter().skip(1).any(|x| *x == g(0))),
                Func::Print => g(2),
                Func::If => 0.0,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn part(name: &str, children: Vec<Part>) -> Part {
        Part { name: name.into(), pose: [0.0; 9], polygons: Vec::new(), children }
    }

    #[test]
    fn parts_resolve_by_alias_case_and_word_order() {
        let model = part(
            "root",
            vec![part("head", Vec::new()), part("left_hind_leg", Vec::new()), part("hat", Vec::new())],
        );
        assert_eq!(resolve(&model, "head").as_deref(), Some("head"));
        assert_eq!(resolve(&model, "headwear").as_deref(), Some("hat"));
        assert_eq!(resolve(&model, "hind_left_leg").as_deref(), Some("left_hind_leg"));
        assert_eq!(resolve(&model, "Left_Hind_Leg").as_deref(), Some("left_hind_leg"));
        assert_eq!(resolve(&model, "tail"), None);
    }

    /// Parses every jem of a real pack (`HORIZON_CEM_PACK=<pack root>`) and prints what fails.
    #[test]
    #[ignore]
    fn real_pack_parses() {
        let Ok(root) = std::env::var("HORIZON_CEM_PACK") else { return };
        let root = PathBuf::from(root);
        let dir = root.join("assets/minecraft/optifine/cem");
        let mut failed = 0;
        let mut total = 0;
        for e in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) != Some("jem") {
                continue;
            }
            let jem: Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
            let mut b = Builder { root: &root, dir: &dir, count: 0, ids: Vec::new(), tex: [64.0, 32.0] };
            let mut model = head();
            let mut names = Vec::new();
            for entry in jem["models"].as_array().into_iter().flatten() {
                let part = b.part(entry, [64.0, 64.0]);
                names.push(part.name.clone());
                model.children.push(part);
            }
            let mut vars = Vec::new();
            for entry in jem["models"].as_array().into_iter().flatten() {
                let resolved = b.resolved(entry);
                for block in resolved["animations"].as_array().into_iter().flatten() {
                    for (k, e) in block.as_object().into_iter().flatten() {
                        total += 1;
                        let text = e.as_str().map(str::to_owned).unwrap_or_else(|| e.to_string());
                        let mut scope = Scope { this: &names[0], part: &names[0], model: &model, ids: &b.ids, vars: &mut vars };
                        if let Err(err) = parse(&text, &mut scope) {
                            failed += 1;
                            if failed < 40 {
                                println!("{}: {k} = {text}: {err}", p.file_name().unwrap().to_string_lossy());
                            }
                        }
                        if scope.target(k).is_none() && failed < 40 {
                            println!("{}: target {k} unresolved", p.file_name().unwrap().to_string_lossy());
                        }
                    }
                }
            }
        }
        println!("{failed} of {total} expressions failed");
    }

    fn head() -> Part {
        Part {
            name: "root".into(),
            pose: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
            polygons: Vec::new(),
            children: vec![Part {
                name: "head".into(),
                pose: [0.0, 6.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
                polygons: cube_polygons([0.0, 0.0], [-4.0, -8.0, -4.0], [8.0; 3], [0.0; 3], false, [64.0, 32.0], &[None; 6]),
                children: Vec::new(),
            }],
        }
    }

    fn extent(p: &Part, base: [f32; 3]) -> (f32, f32) {
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        let at = [base[0] + p.pose[0], base[1] + p.pose[1], base[2] + p.pose[2]];
        for poly in &p.polygons {
            for v in poly.vertices {
                lo = lo.min(at[1] + v[1]);
                hi = hi.max(at[1] + v[1]);
            }
        }
        for c in &p.children {
            let (l, h) = extent(c, at);
            lo = lo.min(l);
            hi = hi.max(h);
        }
        (lo, hi)
    }

    /// A Blockbench creeper head (translate -18, box at 18 in inverted y) lands where vanilla's is.
    #[test]
    fn replaced_head_keeps_vanilla_place() {
        let jem: Value = serde_json::from_str(
            r#"{"textureSize":[64,32],"models":[{"part":"head","id":"head","invertAxis":"xy","translate":[0,-18,0],
               "boxes":[{"coordinates":[-4,18,-4,8,8,8],"textureOffset":[0,0]}],
               "animations":[{"this.rx":"torad(head_pitch) * 2","var.t":"var.t + 1","head.ry":"if(is_on_ground, 1, 2)"}]}]}"#,
        )
        .unwrap();
        let mut model = head();
        let vanilla = extent(&model, [0.0; 3]);
        let (cem, ok) = build(Path::new("/nonexistent"), Path::new("/nonexistent"), &jem, &mut model);
        assert!(ok);
        assert_eq!(extent(&model, [0.0; 3]), vanilla);
        assert_eq!(cem.animations.len(), 3);
        let mut poses = Poses::new(&model);
        let mut values = [0.0; 40];
        values[Param::HeadPitch as usize] = 45.0;
        values[Param::IsOnGround as usize] = 1.0;
        cem.animate(&Inputs { values, id: 7 }, &mut poses);
        let this = &model.children[0].children[0].name;
        assert!((poses.get(this)[3] - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
        assert_eq!(poses.get("head")[4], 1.0);
    }

    #[test]
    fn expressions_follow_optifine_precedence() {
        let model = head();
        let mut vars = Vec::new();
        let mut scope = Scope { this: "head", part: "head", model: &model, ids: &[], vars: &mut vars };
        let e = parse("1 + 2 * 3 - -1 > 7 && !false || clamp(5, 0, 2) == 9", &mut scope).unwrap();
        let mut poses = Poses::new(&model);
        let inp = Inputs { values: [0.0; 40], id: 0 };
        assert_eq!(eval(&e, &inp, &mut poses, &[]), 1.0);
        let e = parse("if(0, 1, 0, 2, 3) + min(4, 2, 8) + fmod(-1, 3)", &mut scope).unwrap();
        assert_eq!(eval(&e, &inp, &mut poses, &[]), 3.0 + 2.0 + 2.0);
    }
}
