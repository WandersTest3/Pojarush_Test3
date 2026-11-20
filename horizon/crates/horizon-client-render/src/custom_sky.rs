//! Custom skies from resource packs: OptiFine's `optifine/sky/world<N>/sky<n>.properties` (and
//! MCPatcher's `mcpatcher/sky/...`), and FabricSkyboxes' `assets/fabricskyboxes/sky/*.json`
//! square-textured skies. Each layer is a texture in a 3 by 2 grid of cube faces drawn around
//! the camera after the sky's colour, faded in and out with the time of day, turned with the sun
//! and blended over the layers before it.
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

/// How a layer blends over what is below it (OptiFine's `blend`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Blend {
    Alpha,
    Add,
    Subtract,
    Multiply,
    Dodge,
    Burn,
    Screen,
    Overlay,
    Replace,
}

impl Blend {
    pub fn parse(name: &str) -> Self {
        match name.trim().to_ascii_lowercase().as_str() {
            "alpha" | "normal" | "decorations" => Self::Alpha,
            "subtract" => Self::Subtract,
            "multiply" => Self::Multiply,
            "dodge" => Self::Dodge,
            "burn" => Self::Burn,
            "screen" => Self::Screen,
            "overlay" => Self::Overlay,
            "replace" | "disable" => Self::Replace,
            _ => Self::Add,
        }
    }
    /// The layer's colour for a brightness (OptiFine's `Blender.setupBlend`).
    pub fn color(self, brightness: f32) -> [f32; 4] {
        let b = brightness.clamp(0.0, 1.0);
        match self {
            Self::Alpha | Self::Add => [1.0, 1.0, 1.0, b],
            Self::Multiply => [b, b, b, b],
            Self::Replace => [1.0, 1.0, 1.0, 1.0],
            _ => [b, b, b, 1.0],
        }
    }
}

/// One sky layer as a pack describes it.
#[derive(Clone, Debug)]
pub struct Layer {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub blend: Blend,
    /// Fade times in ticks of the day (0 is 6:00): start in, end in, start out, end out.
    pub fade: Option<[i32; 4]>,
    pub rotate: bool,
    pub speed: f32,
    pub axis: [f32; 3],
    /// A fixed turn before the rotation with the sun (FabricSkyboxes' `static`), degrees.
    pub fixed: [f32; 3],
    /// Shown in clear weather, rain, thunder.
    pub weather: [bool; 3],
    /// Days of the loop it shows on (empty: every day) and the loop's length.
    pub days: Vec<u32>,
    pub days_loop: u32,
}

/// The layers of each world: 0 the Overworld, 1 the End.
#[derive(Clone, Debug, Default)]
pub struct CustomSky {
    pub worlds: BTreeMap<i32, Vec<Layer>>,
}

/// What draws this frame: the layer's index in [`CustomSky::all`], its colour, and the turn
/// from sky space to camera-relative world.
#[derive(Clone, Copy, Debug)]
pub struct LayerDraw {
    pub index: usize,
    pub blend: Blend,
    pub color: [f32; 4],
    pub rotation: glam::Mat4,
}

/// The largest side a sky texture keeps; larger ones are halved until they fit.
pub const MAX_SIDE: u32 = 4096;

impl CustomSky {
    /// Every layer, in the order [`LayerDraw::index`] counts them.
    pub fn all(&self) -> impl Iterator<Item = &Layer> {
        self.worlds.values().flatten()
    }

    pub fn is_empty(&self) -> bool {
        self.worlds.values().all(Vec::is_empty)
    }

    /// The skies under `root` (a resource root with the packs laid over vanilla).
    pub fn load(root: &Path) -> Self {
        let mut sky = Self::default();
        for world in [0, 1] {
            let mut layers = Vec::new();
            for base in ["optifine", "mcpatcher"] {
                let dir = root.join(format!("assets/minecraft/{base}/sky/world{world}"));
                if !dir.is_dir() {
                    continue;
                }
                for n in 0..100 {
                    let path = dir.join(format!("sky{n}.properties"));
                    let Ok(text) = fs::read_to_string(&path) else {
                        if n > 0 {
                            break;
                        }
                        continue;
                    };
                    if let Some(layer) = optifine_layer(root, &dir, n, &text) {
                        layers.push(layer);
                    }
                }
                if !layers.is_empty() {
                    break;
                }
            }
            if !layers.is_empty() {
                sky.worlds.insert(world, layers);
            }
        }
        for (world, layers) in fabric_skyboxes(root) {
            if !sky.worlds.contains_key(&world) {
                sky.worlds.insert(world, layers);
            }
        }
        sky
    }

    /// The layers to draw for `world` at `day_time` ticks, with `celestial` the sun's angle
    /// (0 to 1), `rain` and `thunder` levels (0 to 1).
    pub fn draws(&self, world: i32, day_time: i64, celestial: f32, rain: f32, thunder: f32) -> Vec<LayerDraw> {
        let mut out = Vec::new();
        let mut index = 0;
        for (w, layers) in &self.worlds {
            for layer in layers {
                if *w == world {
                    let mut brightness = fade(layer.fade, day_time.rem_euclid(24000) as i32);
                    let day = (day_time.div_euclid(24000)).rem_euclid(i64::from(layer.days_loop.max(1))) as u32;
                    if !layer.days.is_empty() && !layer.days.contains(&day) {
                        brightness = 0.0;
                    }
                    let [clear, raining, thundering] = layer.weather;
                    let weather = if thunder > 0.0 && thundering {
                        1.0
                    } else if rain > 0.0 && raining {
                        1.0
                    } else if clear {
                        1.0 - rain
                    } else {
                        rain.max(thunder) * f32::from(u8::from(raining || thundering))
                    };
                    brightness *= weather.clamp(0.0, 1.0);
                    if brightness > 0.001 {
                        out.push(LayerDraw {
                            index,
                            blend: layer.blend,
                            color: layer.blend.color(brightness),
                            rotation: rotation(layer, celestial),
                        });
                    }
                }
                index += 1;
            }
        }
        out
    }
}

/// The layer's turn: its fixed one, then with the sun about its axis.
fn rotation(layer: &Layer, celestial: f32) -> glam::Mat4 {
    let [x, y, z] = layer.fixed.map(f32::to_radians);
    let fixed = glam::Mat4::from_euler(glam::EulerRot::XYZ, x, y, z);
    if !layer.rotate {
        return fixed;
    }
    let axis = glam::Vec3::from_array(layer.axis).normalize_or(glam::Vec3::Z);
    let angle = (celestial * 360.0 * layer.speed).to_radians();
    glam::Mat4::from_axis_angle(axis, angle) * fixed
}

/// OptiFine's fade: 0 to 1 between start and end of fading in, 1 until fading out, then down.
fn fade(times: Option<[i32; 4]>, now: i32) -> f32 {
    let Some([si, ei, so, eo]) = times else {
        return 1.0;
    };
    let since = |from: i32| (now - from).rem_euclid(24000);
    let span = |a: i32, b: i32| (b - a).rem_euclid(24000).max(1);
    if since(si) <= span(si, ei) {
        return since(si) as f32 / span(si, ei) as f32;
    }
    if since(ei) <= span(ei, so) {
        return 1.0;
    }
    if since(so) <= span(so, eo) {
        return 1.0 - since(so) as f32 / span(so, eo) as f32;
    }
    0.0
}

/// `hh:mm` as ticks of the day (6:00 is 0).
fn clock(text: &str) -> Option<i32> {
    let (h, m) = text.trim().split_once(':')?;
    let (h, m): (i32, i32) = (h.trim().parse().ok()?, m.trim().parse().ok()?);
    Some((h * 1000 + m * 1000 / 60 - 6000).rem_euclid(24000))
}

fn properties(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let (k, v) = l.split_once('=').or_else(|| l.split_once(':'))?;
            Some((k.trim().to_owned(), v.trim().to_owned()))
        })
        .collect()
}

/// A path as OptiFine resolves it: `./x` beside the properties, `~/x` under `optifine/`,
/// `ns:path` under `assets/ns/`, else under `assets/minecraft/`.
fn resolve(root: &Path, dir: &Path, path: &str) -> PathBuf {
    let path = path.trim();
    if let Some(rest) = path.strip_prefix("./") {
        return dir.join(rest);
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return root.join("assets/minecraft/optifine").join(rest);
    }
    if let Some((ns, rest)) = path.split_once(':') {
        return root.join("assets").join(ns).join(rest);
    }
    root.join("assets/minecraft").join(path)
}

fn texture(path: &Path) -> Option<(u32, u32, Vec<u8>)> {
    let (mut w, mut h, mut rgba) = crate::hud::decode(path).ok()?;
    while w.max(h) > MAX_SIDE && w >= 2 && h >= 2 {
        (w, h, rgba) = halve(w, h, &rgba);
    }
    Some((w, h, rgba))
}

/// A texture at half its size, each pixel the mean of four.
fn halve(w: u32, h: u32, rgba: &[u8]) -> (u32, u32, Vec<u8>) {
    let (nw, nh) = (w / 2, h / 2);
    let mut out = vec![0u8; (nw * nh * 4) as usize];
    for y in 0..nh {
        for x in 0..nw {
            for c in 0..4 {
                let at = |dx: u32, dy: u32| u32::from(rgba[(((y * 2 + dy) * w + x * 2 + dx) * 4 + c) as usize]);
                out[((y * nw + x) * 4 + c) as usize] = ((at(0, 0) + at(1, 0) + at(0, 1) + at(1, 1)) / 4) as u8;
            }
        }
    }
    (nw, nh, out)
}

fn optifine_layer(root: &Path, dir: &Path, n: u32, text: &str) -> Option<Layer> {
    let p = properties(text);
    let source = match p.get("source") {
        Some(s) => resolve(root, dir, s),
        None => dir.join(format!("sky{n}.png")),
    };
    let (width, height, rgba) = texture(&source)?;
    let si = p.get("startFadeIn").and_then(|t| clock(t));
    let ei = p.get("endFadeIn").and_then(|t| clock(t));
    let eo = p.get("endFadeOut").and_then(|t| clock(t));
    let fade = match (si, ei, eo) {
        (Some(si), Some(ei), Some(eo)) => {
            let so = p
                .get("startFadeOut")
                .and_then(|t| clock(t))
                .unwrap_or_else(|| (eo - (ei - si).rem_euclid(24000)).rem_euclid(24000));
            Some([si, ei, so, eo])
        }
        _ => None,
    };
    let axis = p
        .get("axis")
        .map(|a| {
            let v: Vec<f32> = a.split_whitespace().filter_map(|x| x.parse().ok()).collect();
            if v.len() == 3 && v.iter().any(|c| *c != 0.0) { [v[0], v[1], v[2]] } else { [0.0, 0.0, 1.0] }
        })
        .unwrap_or([0.0, 0.0, 1.0]);
    let weather = match p.get("weather") {
        Some(w) => {
            let has = |k: &str| w.split_whitespace().any(|x| x.eq_ignore_ascii_case(k));
            [has("clear"), has("rain"), has("thunder")]
        }
        None => [true, false, false],
    };
    let days = p.get("days").map(|d| days(d)).unwrap_or_default();
    Some(Layer {
        width,
        height,
        rgba,
        blend: Blend::parse(p.get("blend").map_or("add", String::as_str)),
        fade,
        rotate: p.get("rotate").is_none_or(|r| !r.eq_ignore_ascii_case("false")),
        speed: p.get("speed").and_then(|s| s.parse().ok()).filter(|s: &f32| s.is_finite()).unwrap_or(1.0),
        axis,
        fixed: [0.0; 3],
        weather,
        days,
        days_loop: p.get("daysLoop").and_then(|d| d.parse().ok()).filter(|d| *d > 0).unwrap_or(8),
    })
}

/// `0 2-4 6` as a list of days.
fn days(text: &str) -> Vec<u32> {
    let mut out = Vec::new();
    for part in text.split_whitespace() {
        match part.split_once('-') {
            Some((a, b)) => {
                if let (Ok(a), Ok(b)) = (a.parse::<u32>(), b.parse::<u32>()) {
                    out.extend(a.min(b)..=b.max(a).min(a.min(b) + 1000));
                }
            }
            None => out.extend(part.parse::<u32>().ok()),
        }
    }
    out
}

/// FabricSkyboxes' square-textured skies, by world.
fn fabric_skyboxes(root: &Path) -> BTreeMap<i32, Vec<Layer>> {
    let mut found: Vec<(i64, String, i32, Layer)> = Vec::new();
    for ns in ["fabricskyboxes", "nuit"] {
        let dir = root.join(format!("assets/{ns}/sky"));
        let Ok(entries) = fs::read_dir(&dir) else { continue };
        let mut files: Vec<PathBuf> = entries.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "json")).collect();
        files.sort();
        for path in files {
            let Ok(bytes) = fs::read(&path) else { continue };
            let Ok(json) = serde_json::from_slice::<serde_json::Value>(&bytes) else { continue };
            if let Some((priority, worlds, layer)) = fabric_layer(root, &json) {
                let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                for world in worlds {
                    found.push((priority, name.clone(), world, layer.clone()));
                }
            }
        }
    }
    found.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    let mut out: BTreeMap<i32, Vec<Layer>> = BTreeMap::new();
    for (_, _, world, layer) in found {
        out.entry(world).or_default().push(layer);
    }
    out
}

fn fabric_layer(root: &Path, json: &serde_json::Value) -> Option<(i64, Vec<i32>, Layer)> {
    use serde_json::Value;
    let kind = json.get("type")?.as_str()?;
    if !matches!(kind, "square-textured" | "square_textured" | "multi-textured") {
        return None;
    }
    let tex_path = |s: &str| {
        let (ns, rest) = s.split_once(':').unwrap_or(("minecraft", s));
        root.join("assets").join(ns).join(rest)
    };
    let (width, height, rgba) = if let Some(t) = json.get("texture").and_then(Value::as_str) {
        texture(&tex_path(t))?
    } else {
        let faces = json.get("textures")?;
        let order = ["bottom", "north", "south", "top", "east", "west"];
        let mut images = Vec::new();
        for name in order {
            images.push(texture(&tex_path(faces.get(name)?.as_str()?))?);
        }
        let side = images.iter().map(|(w, h, _)| (*w).max(*h)).max()?.min(MAX_SIDE / 3);
        let (w, h) = (side * 3, side * 2);
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        let cells = [(0, 0), (1, 1), (2, 0), (1, 0), (2, 1), (0, 1)];
        for ((fw, fh, data), (cx, cy)) in images.iter().zip(cells) {
            for y in 0..side {
                for x in 0..side {
                    let sx = x * fw / side;
                    let sy = y * fh / side;
                    let from = ((sy * fw + sx) * 4) as usize;
                    let to = (((cy * side + y) * w + cx * side + x) * 4) as usize;
                    rgba[to..to + 4].copy_from_slice(&data[from..from + 4]);
                }
            }
        }
        (w, h, rgba)
    };
    let props = json.get("properties").cloned().unwrap_or(Value::Null);
    let num = |v: Option<&Value>| v.and_then(Value::as_f64);
    let fade = props.get("fade").and_then(|f| {
        if f.get("alwaysOn").and_then(Value::as_bool) == Some(true) {
            return None;
        }
        let t = |k: &str| num(f.get(k)).map(|v| (v as i64).rem_euclid(24000) as i32);
        Some([t("startFadeIn")?, t("endFadeIn")?, t("startFadeOut")?, t("endFadeOut")?])
    });
    let vec3 = |v: Option<&Value>| {
        v.map(|v| {
            [
                num(v.get("x")).unwrap_or(0.0) as f32,
                num(v.get("y")).unwrap_or(0.0) as f32,
                num(v.get("z")).unwrap_or(0.0) as f32,
            ]
        })
    };
    let rotation = props.get("rotation");
    let fixed = vec3(rotation.and_then(|r| r.get("static"))).unwrap_or([0.0; 3]);
    let speed = num(rotation.and_then(|r| r.get("rotationSpeed"))).unwrap_or(1.0) as f32;
    let rotate = rotation.and_then(|r| r.get("axis")).is_some() && speed != 0.0;
    let blend = json
        .get("blend")
        .and_then(|b| b.get("type"))
        .and_then(Value::as_str)
        .map_or(Blend::Alpha, Blend::parse);
    let conditions = json.get("conditions");
    let strings = |key: &str| -> Vec<String> {
        conditions
            .and_then(|c| c.get(key))
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_str).map(str::to_owned).collect())
            .unwrap_or_default()
    };
    let worlds: Vec<i32> = {
        let w = strings("worlds");
        if w.is_empty() {
            vec![0]
        } else {
            w.iter()
                .filter_map(|w| match w.as_str() {
                    "minecraft:overworld" | "overworld" => Some(0),
                    "minecraft:the_end" | "the_end" => Some(1),
                    _ => None,
                })
                .collect()
        }
    };
    let weather = {
        let w = strings("weather");
        if w.is_empty() {
            [true, true, true]
        } else {
            let has = |k: &str| w.iter().any(|x| x == k);
            [has("clear"), has("rain"), has("thunder")]
        }
    };
    let priority = num(props.get("priority")).unwrap_or(0.0) as i64;
    Some((
        priority,
        worlds,
        Layer {
            width,
            height,
            rgba,
            blend,
            fade,
            rotate,
            speed,
            axis: [0.0, 0.0, 1.0],
            fixed,
            weather,
            days: Vec::new(),
            days_loop: 8,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_counts_from_sunrise() {
        assert_eq!(clock("6:00"), Some(0));
        assert_eq!(clock("12:00"), Some(6000));
        assert_eq!(clock("0:00"), Some(18000));
        assert_eq!(clock("18:00"), Some(12000));
    }

    #[test]
    fn fade_rises_holds_and_falls() {
        let t = Some([0, 1000, 11000, 12000]);
        assert_eq!(fade(t, 0), 0.0);
        assert!((fade(t, 500) - 0.5).abs() < 0.01);
        assert_eq!(fade(t, 5000), 1.0);
        assert!((fade(t, 11500) - 0.5).abs() < 0.01);
        assert_eq!(fade(t, 18000), 0.0);
        assert_eq!(fade(None, 18000), 1.0);
    }

    #[test]
    fn days_list_and_ranges() {
        assert_eq!(days("0 2-4 6"), vec![0, 2, 3, 4, 6]);
    }
}
