use horizon_mod_sdk::{
    Mod, Registration, game,
    render::{self, WorldBox, WorldLine, WorldPoint, WorldPos, WorldShape},
    storage,
    ui::{self, ButtonWidget, SliderWidget, TextWidget, WidgetV2 as Widget},
};

const FILE: &str = "combat-hitboxes.txt";
const REACH: f32 = 64.0;

const PALETTE: [(u32, [&str; 3]); 12] = [
    (0xFFFFFFFF, ["White", "Branco", "Branco"]),
    (0xFFFF0000, ["Red", "Vermelho", "Vermelho"]),
    (0xFF0000FF, ["Blue", "Azul", "Azul"]),
    (0xFFFF00FF, ["Magenta", "Magenta", "Magenta"]),
    (0xFF000000, ["Black", "Preto", "Preto"]),
    (0xFF00FF00, ["Green", "Verde", "Verde"]),
    (0xFFFFFF00, ["Yellow", "Amarelo", "Amarelo"]),
    (0xFF00FFFF, ["Cyan", "Ciano", "Ciano"]),
    (0xFFFFA500, ["Orange", "Laranja", "Laranja"]),
    (0xFF57197F, ["Purple", "Roxo", "Roxo"]),
    (0xFFFF69B4, ["Pink", "Rosa", "Rosa"]),
    (0xFF808080, ["Gray", "Cinza", "Cinzento"]),
];

#[derive(Clone, Copy)]
enum Kind {
    Toggle,
    Color,
    Slider { min: f32, max: f32, steps: u32 },
}

struct Setting {
    key: &'static str,
    kind: Kind,
    default: f32,
    label: [&'static str; 3],
}

const SETTINGS: [Setting; 19] = [
    Setting { key: "enabled", kind: Kind::Toggle, default: 1.0, label: ["Enabled", "Ativado", "Ativado"] },
    Setting { key: "hideFireworks", kind: Kind::Toggle, default: 0.0, label: ["Hide fireworks", "Ocultar fogos de artifício", "Ocultar foguetes"] },
    Setting { key: "hideItems", kind: Kind::Toggle, default: 0.0, label: ["Hide dropped items", "Ocultar itens no chão", "Ocultar itens no chão"] },
    Setting { key: "hitBoxColor", kind: Kind::Color, default: 0.0, label: ["Hitbox colour", "Cor da hitbox", "Cor da hitbox"] },
    Setting { key: "changeTargetColor", kind: Kind::Toggle, default: 1.0, label: ["Colour the target", "Colorir o alvo", "Colorir o alvo"] },
    Setting { key: "targetBoxColor", kind: Kind::Color, default: 1.0, label: ["Target colour", "Cor do alvo", "Cor do alvo"] },
    Setting { key: "hitBoxHurt", kind: Kind::Toggle, default: 0.0, label: ["Colour when hurt", "Colorir ao levar dano", "Colorir ao levar dano"] },
    Setting { key: "hurtBoxColor", kind: Kind::Color, default: 3.0, label: ["Hurt colour", "Cor do dano", "Cor do dano"] },
    Setting { key: "renderEyeHeight", kind: Kind::Toggle, default: 1.0, label: ["Eye height", "Altura dos olhos", "Altura dos olhos"] },
    Setting { key: "eyeColor", kind: Kind::Color, default: 1.0, label: ["Eye colour", "Cor dos olhos", "Cor dos olhos"] },
    Setting { key: "renderLookDir", kind: Kind::Toggle, default: 1.0, label: ["Look direction", "Direção do olhar", "Direção do olhar"] },
    Setting { key: "lineLookDir", kind: Kind::Toggle, default: 1.0, label: ["Look as a line", "Olhar como linha", "Olhar como linha"] },
    Setting { key: "lookColor", kind: Kind::Color, default: 2.0, label: ["Look colour", "Cor do olhar", "Cor do olhar"] },
    Setting { key: "line1", kind: Kind::Slider { min: 0.5, max: 10.0, steps: 19 }, default: 2.5, label: ["Line width", "Espessura da linha", "Espessura da linha"] },
    Setting { key: "distFor2", kind: Kind::Slider { min: 1.0, max: 64.0, steps: 63 }, default: 32.0, label: ["Far from", "Longe a partir de", "Longe a partir de"] },
    Setting { key: "line2", kind: Kind::Slider { min: 0.5, max: 10.0, steps: 19 }, default: 2.5, label: ["Far line width", "Espessura de longe", "Espessura ao longe"] },
    Setting { key: "outlineEnabled", kind: Kind::Toggle, default: 0.0, label: ["Outline", "Contorno", "Contorno"] },
    Setting { key: "outlineColor", kind: Kind::Color, default: 4.0, label: ["Outline colour", "Cor do contorno", "Cor do contorno"] },
    Setting { key: "outlineMultiplier", kind: Kind::Slider { min: 1.0, max: 4.0, steps: 12 }, default: 2.0, label: ["Outline width", "Espessura do contorno", "Espessura do contorno"] },
];

const RESET_ALL: u32 = 100;
const INFO: u32 = 101;

fn lang() -> usize {
    match ui::language().as_str() {
        "pt_br" => 1,
        "pt_pt" => 2,
        _ => 0,
    }
}

struct Hitboxes {
    values: [f32; SETTINGS.len()],
}

impl Hitboxes {
    fn index(key: &str) -> usize {
        SETTINGS.iter().position(|s| s.key == key).unwrap_or(0)
    }

    fn value(&self, key: &str) -> f32 {
        self.values[Self::index(key)]
    }

    fn on(&self, key: &str) -> bool {
        self.value(key) >= 0.5
    }

    fn color(&self, key: &str) -> u32 {
        PALETTE[(self.value(key) as usize).min(PALETTE.len() - 1)].0
    }

    fn load() -> Self {
        let mut values = SETTINGS.map(|s| s.default);
        if let Some(text) = storage::get(FILE).and_then(|v| String::from_utf8(v).ok()) {
            for line in text.lines() {
                let Some((k, v)) = line.split_once('=') else { continue };
                let Some(i) = SETTINGS.iter().position(|s| s.key == k.trim()) else { continue };
                let Ok(v) = v.trim().parse::<f32>() else { continue };
                if v.is_finite() {
                    values[i] = Self::clamp(i, v);
                }
            }
        }
        Self { values }
    }

    fn clamp(i: usize, v: f32) -> f32 {
        match SETTINGS[i].kind {
            Kind::Toggle => if v >= 0.5 { 1.0 } else { 0.0 },
            Kind::Color => (v.round()).clamp(0.0, (PALETTE.len() - 1) as f32),
            Kind::Slider { min, max, .. } => v.clamp(min, max),
        }
    }

    fn save(&self) {
        let text: String = SETTINGS
            .iter()
            .zip(self.values)
            .map(|(s, v)| format!("{}={v:.2}\n", s.key))
            .collect();
        let _ = storage::set(FILE, text.as_bytes());
    }

    fn page(&self) {
        let l = lang();
        let on_off = |b: bool| match (l, b) {
            (0, true) => "On",
            (0, false) => "Off",
            (_, true) => "Ligado",
            (_, false) => "Desligado",
        };
        let mut widgets = vec![Widget::Text(TextWidget {
            id: INFO,
            text: ["Shown with F3+B.", "Aparece com F3+B.", "Aparece com F3+B."][l].into(),
            color: 0xB98CF2,
        })];
        for (i, s) in SETTINGS.iter().enumerate() {
            let id = i as u32;
            let v = self.values[i];
            let label = s.label[l];
            widgets.push(match s.kind {
                Kind::Toggle => Widget::Button(ButtonWidget { id, label: format!("{label}: {}", on_off(v >= 0.5)) }),
                Kind::Color => Widget::Button(ButtonWidget { id, label: format!("{label}: {}", PALETTE[v as usize].1[l]) }),
                Kind::Slider { min, max, steps } => Widget::Slider(SliderWidget {
                    id,
                    label: format!("{label}: {v:.2}"),
                    steps,
                    position: (((v - min) / (max - min)) * steps as f32).round() as u32,
                }),
            });
        }
        widgets.push(Widget::Button(ButtonWidget {
            id: RESET_ALL,
            label: ["Reset all", "Restaurar tudo", "Repor tudo"][l].into(),
        }));
        ui::set_settings_v2(&widgets);
        let resets: Vec<u32> = (0..SETTINGS.len() as u32).collect();
        ui::set_resets(&resets);
    }

    fn draw(&self) {
        if !self.on("enabled") || !game::hitboxes_shown() {
            render::set_world_shapes(&[], &[]);
            return;
        }
        let entities = game::entities(REACH);
        let Some(player) = game::get_player() else {
            render::set_world_shapes(&[], &[]);
            return;
        };
        let mut shapes = Vec::new();
        let pos = |x: f64, y: f64, z: f64| WorldPos { x, y, z };
        for e in &entities {
            if self.on("hideFireworks") && e.kind == "minecraft:firework_rocket" {
                continue;
            }
            if self.on("hideItems") && e.kind == "minecraft:item" {
                continue;
            }
            let min = pos(e.min_x, e.min_y, e.min_z);
            let max = pos(e.max_x, e.max_y, e.max_z);
            if e.kind == "minecraft:ender_dragon_part" {
                shapes.push(WorldShape::Cuboid(WorldBox { min, max, color: 0xFF3FFF00, width: 2.5 }));
                continue;
            }
            let distance = ((e.x - player.x).powi(2) + (e.y - player.y).powi(2) + (e.z - player.z).powi(2)).sqrt();
            let width = if distance > f64::from(self.value("distFor2")) { self.value("line2") } else { self.value("line1") };
            let color = if e.living && e.hurt_time != 0 && self.on("hitBoxHurt") {
                self.color("hurtBoxColor")
            } else if self.on("changeTargetColor") && e.targeted {
                self.color("targetBoxColor")
            } else {
                self.color("hitBoxColor")
            };
            if self.on("outlineEnabled") {
                shapes.push(WorldShape::Cuboid(WorldBox {
                    min,
                    max,
                    color: self.color("outlineColor"),
                    width: width * self.value("outlineMultiplier"),
                }));
            }
            shapes.push(WorldShape::Cuboid(WorldBox { min, max, color, width }));
            shapes.push(WorldShape::Point(WorldPoint { at: pos(e.x, e.y, e.z), color, size: 2.0 }));
            if let Some(vehicle) = e.vehicle.and_then(|v| entities.iter().find(|o| o.id == v && v != 0)) {
                let half = ((vehicle.max_x - vehicle.min_x).min(e.max_x - e.min_x) / 2.0) as f64;
                shapes.push(WorldShape::Cuboid(WorldBox {
                    min: pos(e.x - half, e.y, e.z - half),
                    max: pos(e.x + half, e.y + 0.0625, e.z + half),
                    color: 0xFFFFFF00,
                    width,
                }));
            }
            if e.living && self.on("renderEyeHeight") {
                let eye = e.min_y + f64::from(e.eye_height);
                shapes.push(WorldShape::Cuboid(WorldBox {
                    min: pos(e.min_x, eye - 0.01, e.min_z),
                    max: pos(e.max_x, eye + 0.01, e.max_z),
                    color: self.color("eyeColor"),
                    width,
                }));
            }
            if self.on("renderLookDir") {
                let start = pos(e.x, e.y + f64::from(e.eye_height), e.z);
                let (yaw, pitch) = (f64::from(e.yaw).to_radians(), f64::from(e.pitch).to_radians());
                let (dx, dy, dz) = (-yaw.sin() * pitch.cos(), -pitch.sin(), yaw.cos() * pitch.cos());
                let end = pos(start.x + dx * 2.0, start.y + dy * 2.0, start.z + dz * 2.0);
                let line = WorldLine { start, end, color: self.color("lookColor"), width };
                shapes.push(if self.on("lineLookDir") { WorldShape::Line(line) } else { WorldShape::Arrow(line) });
            }
        }
        render::set_world_shapes(&shapes, &[]);
    }
}

impl Mod for Hitboxes {
    fn init() -> Result<(Self, Registration), String> {
        let mod_ = Hitboxes::load();
        render::hide_hitboxes(true);
        mod_.page();
        Ok((mod_, Registration::new().on_frame()))
    }

    fn frame(&mut self, _seconds: f32) {
        self.draw();
    }

    fn ui_select(&mut self, id: u32, row: u32) {
        if id == RESET_ALL {
            self.values = SETTINGS.map(|s| s.default);
        } else if let Some(s) = SETTINGS.get(id as usize) {
            let i = id as usize;
            self.values[i] = if row == u32::MAX {
                s.default
            } else {
                match s.kind {
                    Kind::Toggle => if self.values[i] >= 0.5 { 0.0 } else { 1.0 },
                    Kind::Color => ((self.values[i] as usize + 1) % PALETTE.len()) as f32,
                    Kind::Slider { min, max, steps } => min + (max - min) * row.min(steps) as f32 / steps as f32,
                }
            };
        } else {
            return;
        }
        self.save();
        self.page();
    }

    fn ui_settings_opened(&mut self) {
        self.page();
    }
}

horizon_mod_sdk::export_mod!(Hitboxes);
