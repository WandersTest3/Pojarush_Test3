use horizon_mod_sdk::{
    Mod, Registration,
    render::{self, ModelPlace},
    storage,
    ui::{self, ButtonWidget, SliderWidget, TextWidget, WidgetV2},
};
use std::fmt::Write;

const FILE: &str = "config.txt";
const ENABLED: u32 = 1;
const LAYERS: u32 = 10;
const S_HEAD: u32 = 20;
const S_BODY: u32 = 21;
const S_BASE: u32 = 22;
const RESET: u32 = 30;
const SIZE_MIN: u32 = 100;
const SIZE_MAX: u32 = 150;

struct Layer {
    key: &'static str,
    part: &'static str,
    hide: &'static str,
    uv: [u32; 2],
    from: [f32; 3],
    from_slim: [f32; 3],
    size: [u32; 3],
    slim_width: u32,
    kind: Kind,
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Head,
    Body,
    Limb,
}

const LAYER_LIST: [Layer; 6] = [
    Layer { key: "hat", part: "head", hide: "hat", uv: [32, 0], from: [-4.0, -8.0, -4.0], from_slim: [-4.0, -8.0, -4.0], size: [8, 8, 8], slim_width: 8, kind: Kind::Head },
    Layer { key: "jacket", part: "body", hide: "jacket", uv: [16, 32], from: [-4.0, 0.0, -2.0], from_slim: [-4.0, 0.0, -2.0], size: [8, 12, 4], slim_width: 8, kind: Kind::Body },
    Layer { key: "left_sleeve", part: "left_arm", hide: "left_sleeve", uv: [48, 48], from: [-1.0, -2.0, -2.0], from_slim: [-1.0, -2.0, -2.0], size: [4, 12, 4], slim_width: 3, kind: Kind::Limb },
    Layer { key: "right_sleeve", part: "right_arm", hide: "right_sleeve", uv: [40, 32], from: [-3.0, -2.0, -2.0], from_slim: [-2.0, -2.0, -2.0], size: [4, 12, 4], slim_width: 3, kind: Kind::Limb },
    Layer { key: "left_pants", part: "left_leg", hide: "left_pants", uv: [0, 48], from: [-2.0, 0.0, -2.0], from_slim: [-2.0, 0.0, -2.0], size: [4, 12, 4], slim_width: 4, kind: Kind::Limb },
    Layer { key: "right_pants", part: "right_leg", hide: "right_pants", uv: [0, 32], from: [-2.0, 0.0, -2.0], from_slim: [-2.0, 0.0, -2.0], size: [4, 12, 4], slim_width: 4, kind: Kind::Limb },
];

fn strings() -> [&'static str; 16] {
    match ui::language().as_str() {
        "pt_br" => [
            "Ativado",
            "Desativado",
            "3D Skin Layers",
            "Chapéu",
            "Jaqueta",
            "Manga esquerda",
            "Manga direita",
            "Calça esquerda",
            "Calça direita",
            "Tamanho dos voxels da cabeça",
            "Largura dos voxels do corpo",
            "Tamanho dos voxels dos membros",
            "Restaurar padrões",
            "Porte de teste não oficial do 3D Skin Layers, de tr7zw. Todo o crédito pela ideia e pelo design original é dele.",
            "Aparece no seu jogador em terceira pessoa.",
            "Não redistribua.",
        ],
        "pt_pt" => [
            "Ativado",
            "Desativado",
            "3D Skin Layers",
            "Chapéu",
            "Casaco",
            "Manga esquerda",
            "Manga direita",
            "Calças (esquerda)",
            "Calças (direita)",
            "Tamanho dos voxels da cabeça",
            "Largura dos voxels do corpo",
            "Tamanho dos voxels dos membros",
            "Repor predefinições",
            "Port de teste não oficial do 3D Skin Layers, de tr7zw. Todo o crédito pela ideia e pelo design originais é dele.",
            "Aparece no teu jogador na terceira pessoa.",
            "Não redistribuas.",
        ],
        _ => [
            "On",
            "Off",
            "3D Skin Layers",
            "Hat",
            "Jacket",
            "Left Sleeve",
            "Right Sleeve",
            "Left Pants",
            "Right Pants",
            "Head Voxel Size",
            "Body Voxel Width",
            "Limb Voxel Size",
            "Reset to Defaults",
            "Unofficial test port of 3D Skin Layers by tr7zw. All credit for the original idea and design goes to tr7zw.",
            "Shown on your own player in third person.",
            "Not for redistribution.",
        ],
    }
}

struct Skin {
    pixels: Vec<u8>,
    slim: bool,
}

impl Skin {
    fn alpha(&self, x: u32, y: u32) -> u8 {
        self.pixels.get(((y * 64 + x) * 4 + 3) as usize).copied().unwrap_or(0)
    }
}

struct Layers {
    enabled: bool,
    layers: [bool; 6],
    head: u32,
    body: u32,
    base: u32,
    skin: Option<Skin>,
    models: Vec<u32>,
    dirty: bool,
}

fn stored(cfg: &[(String, String)], key: &str, default: u32, min: u32, max: u32) -> u32 {
    cfg.iter()
        .find(|(k, _)| k == key)
        .and_then(|(_, v)| v.parse().ok())
        .map_or(default, |v: u32| v.clamp(min, max))
}

fn voxels(layer: &Layer, skin: &Skin) -> Vec<([i32; 3], [u32; 2])> {
    let [u, v] = layer.uv;
    let [w0, h, d] = layer.size;
    let w = if skin.slim { layer.slim_width } else { w0 };
    let (w, h, d) = (w as i32, h as i32, d as i32);
    let (u, v) = (u as i32, v as i32);
    let mut out: Vec<([i32; 3], [u32; 2])> = Vec::new();
    let mut put = |p: [i32; 3], px: i32, py: i32| {
        if px < 0 || py < 0 || px >= 64 || py >= 64 || skin.alpha(px as u32, py as u32) == 0 {
            return;
        }
        if !out.iter().any(|(q, _)| *q == p) {
            out.push((p, [px as u32, py as u32]));
        }
    };
    for j in 0..h {
        for i in 0..w {
            put([i, j, 0], u + d + i, v + d + j);
            put([w - 1 - i, j, d - 1], u + 2 * d + w + i, v + d + j);
        }
        for i in 0..d {
            put([0, j, d - 1 - i], u + i, v + d + j);
            put([w - 1, j, i], u + d + w + i, v + d + j);
        }
    }
    for j in 0..d {
        for i in 0..w {
            put([i, 0, d - 1 - j], u + d + i, v + j);
            put([i, h - 1, d - 1 - j], u + d + w + i, v + j);
        }
    }
    out
}

fn f(x: f32) -> String {
    let r = (x * 10000.0).round() / 10000.0;
    if r == r.trunc() { format!("{r:.1}") } else { format!("{r}") }
}

impl Layers {
    fn scale(&self, kind: Kind) -> [f32; 3] {
        let head = self.head as f32 / 100.0;
        let body = self.body as f32 / 100.0;
        let base = self.base as f32 / 100.0;
        let tall = 1.0 + (base - 1.0) * 0.25;
        match kind {
            Kind::Head => [head; 3],
            Kind::Body => [body, tall, base],
            Kind::Limb => [base, tall, base],
        }
    }

    fn geometry(&self, layer: &Layer, skin: &Skin) -> Option<String> {
        let cells = voxels(layer, skin);
        if cells.is_empty() {
            return None;
        }
        let w = if skin.slim { layer.slim_width } else { layer.size[0] } as f32;
        let from = if skin.slim { layer.from_slim } else { layer.from };
        let size = [w, layer.size[1] as f32, layer.size[2] as f32];
        let center = [from[0] + size[0] / 2.0, from[1] + size[1] / 2.0, from[2] + size[2] / 2.0];
        let k = self.scale(layer.kind);
        let has = |p: [i32; 3]| cells.iter().any(|(q, _)| *q == p);
        let mut cubes = String::new();
        for (p, [px, py]) in &cells {
            let min: [f32; 3] = std::array::from_fn(|a| center[a] + (from[a] + p[a] as f32 - center[a]) * k[a]);
            let neighbours = [
                ("down", [0, -1, 0]),
                ("up", [0, 1, 0]),
                ("east", [-1, 0, 0]),
                ("north", [0, 0, -1]),
                ("west", [1, 0, 0]),
                ("south", [0, 0, 1]),
            ];
            let mut faces = String::new();
            for (name, o) in neighbours {
                if has([p[0] + o[0], p[1] + o[1], p[2] + o[2]]) {
                    continue;
                }
                if !faces.is_empty() {
                    faces.push(',');
                }
                let _ = write!(
                    faces,
                    "\"{name}\":{{\"uv\":[{},{}],\"uv_size\":[0.8,0.8]}}",
                    f(*px as f32 + 0.1),
                    f(*py as f32 + 0.1)
                );
            }
            if faces.is_empty() {
                continue;
            }
            if !cubes.is_empty() {
                cubes.push(',');
            }
            let _ = write!(
                cubes,
                "{{\"origin\":[{},{},{}],\"size\":[{},{},{}],\"uv\":{{{faces}}}}}",
                f(-(min[0] + k[0])),
                f(24.0 - (min[1] + k[1])),
                f(min[2]),
                f(k[0]),
                f(k[1]),
                f(k[2])
            );
        }
        Some(format!(
            "{{\"format_version\":\"1.12.0\",\"minecraft:geometry\":[{{\"description\":{{\"identifier\":\"geometry.skin_layer\",\"texture_width\":64,\"texture_height\":64}},\"bones\":[{{\"name\":\"layer\",\"pivot\":[0,24,0],\"cubes\":[{cubes}]}}]}}]}}"
        ))
    }

    fn clear(&mut self) {
        for id in self.models.drain(..) {
            render::remove_model(id);
        }
        render::hide_player_parts(&[]);
    }

    fn rebuild(&mut self) {
        self.dirty = false;
        self.clear();
        if !self.enabled {
            return;
        }
        let Some(skin) = self.skin.as_ref() else { return };
        let mut hidden = Vec::new();
        let mut made = Vec::new();
        for (i, layer) in LAYER_LIST.iter().enumerate() {
            if !self.layers[i] {
                continue;
            }
            hidden.push(layer.hide.to_string());
            let Some(json) = self.geometry(layer, skin) else { continue };
            let place = ModelPlace { x: 0.0, y: 0.0, z: 0.0, yaw: 0.0, pitch: 0.0, scale: 1.0 };
            if let Ok(id) = render::spawn_model_json(&json, "player:skin", place) {
                render::attach_model(id, layer.part);
                made.push(id);
            }
        }
        self.models = made;
        render::hide_player_parts(&hidden);
    }

    fn poll_skin(&mut self) {
        let pixels = render::player_skin().filter(|p| p.len() == 64 * 64 * 4);
        let slim = render::player_skin_slim();
        let same = match (&self.skin, &pixels) {
            (Some(s), Some(p)) => s.slim == slim && s.pixels == *p,
            (None, None) => true,
            _ => false,
        };
        if !same {
            self.skin = pixels.map(|pixels| Skin { pixels, slim });
            self.dirty = true;
        }
    }

    fn save(&self) {
        let mut text = format!("enabled={}\n", u32::from(self.enabled));
        for (i, l) in LAYER_LIST.iter().enumerate() {
            let _ = writeln!(text, "{}={}", l.key, u32::from(self.layers[i]));
        }
        let _ = write!(text, "head={}\nbody={}\nbase={}\n", self.head, self.body, self.base);
        let _ = storage::set(FILE, text.as_bytes());
    }

    fn slider(&self, id: u32) -> (String, u32) {
        let t = strings();
        let (name, value) = match id {
            S_HEAD => (t[9], self.head),
            S_BODY => (t[10], self.body),
            _ => (t[11], self.base),
        };
        (format!("{name}: {}.{:02}", value / 100, value % 100), value - SIZE_MIN)
    }

    fn page(&self) {
        let t = strings();
        let on = |b: bool| if b { t[0] } else { t[1] };
        let mut widgets = vec![
            WidgetV2::Text(TextWidget { id: 0, text: t[13].into(), color: 0xB98CF2 }),
            WidgetV2::Text(TextWidget { id: 0, text: format!("{} {}", t[14], t[15]), color: 0xAAAAAA }),
            WidgetV2::Button(ButtonWidget { id: ENABLED, label: format!("{}: {}", t[2], on(self.enabled)) }),
        ];
        for (i, _) in LAYER_LIST.iter().enumerate() {
            widgets.push(WidgetV2::Button(ButtonWidget {
                id: LAYERS + i as u32,
                label: format!("{}: {}", t[3 + i], on(self.layers[i])),
            }));
        }
        for id in [S_HEAD, S_BODY, S_BASE] {
            let (label, position) = self.slider(id);
            widgets.push(WidgetV2::Slider(SliderWidget { id, label, steps: SIZE_MAX - SIZE_MIN, position }));
        }
        widgets.push(WidgetV2::Button(ButtonWidget { id: RESET, label: t[12].into() }));
        ui::set_settings_v2(&widgets);
    }

    fn defaults(&mut self) {
        self.enabled = true;
        self.layers = [true; 6];
        self.head = 118;
        self.body = 105;
        self.base = 115;
    }
}

impl Mod for Layers {
    fn init() -> Result<(Self, Registration), String> {
        let cfg: Vec<(String, String)> = storage::get(FILE)
            .and_then(|v| String::from_utf8(v).ok())
            .map(|t| {
                t.lines()
                    .filter_map(|l| l.split_once('='))
                    .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
                    .collect()
            })
            .unwrap_or_default();
        let layers = std::array::from_fn(|i| stored(&cfg, LAYER_LIST[i].key, 1, 0, 1) == 1);
        let m = Layers {
            enabled: stored(&cfg, "enabled", 1, 0, 1) == 1,
            layers,
            head: stored(&cfg, "head", 118, SIZE_MIN, SIZE_MAX),
            body: stored(&cfg, "body", 105, SIZE_MIN, SIZE_MAX),
            base: stored(&cfg, "base", 115, SIZE_MIN, SIZE_MAX),
            skin: None,
            models: Vec::new(),
            dirty: true,
        };
        m.page();
        Ok((m, Registration::new().on_tick()))
    }

    fn tick(&mut self, tick: u64) {
        if tick % 20 == 0 || self.skin.is_none() {
            self.poll_skin();
        }
        if self.dirty {
            self.rebuild();
        }
    }

    fn ui_select(&mut self, id: u32, row: u32) {
        match id {
            S_HEAD | S_BODY | S_BASE => {
                let value = (SIZE_MIN + row).min(SIZE_MAX);
                match id {
                    S_HEAD => self.head = value,
                    S_BODY => self.body = value,
                    _ => self.base = value,
                }
                let (label, position) = self.slider(id);
                ui::set_slider(id, position, &label);
                self.save();
                self.dirty = true;
                return;
            }
            ENABLED => self.enabled = !self.enabled,
            RESET => self.defaults(),
            i if (LAYERS..LAYERS + 6).contains(&i) => {
                let i = (i - LAYERS) as usize;
                self.layers[i] = !self.layers[i];
            }
            _ => return,
        }
        self.save();
        self.dirty = true;
        self.page();
    }

    fn ui_settings_opened(&mut self) {
        self.page();
    }
}

horizon_mod_sdk::export_mod!(Layers);

