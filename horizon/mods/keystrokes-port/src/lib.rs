use horizon_mod_sdk::{
    Mod, Registration, input,
    render::{self, Corner, FillShape, Point, Rect, Shape, TextShape},
    storage,
    ui::{self, ButtonWidget, WidgetV2 as Widget},
};

const FILE: &str = "keystrokes.txt";
const BACKGROUND: u32 = 0x7800_0000;
const HELD: u32 = 0xB0FF_FFFF;
const INK_HELD: u32 = 0x14_1414;
const INK: u32 = 0xFF_FFFF;
const KEY: f32 = 22.0;
const GAP: f32 = 2.0;
const SIZE: (f32, f32) = (3.0 * KEY + 2.0 * GAP, 3.0 * KEY + 3.0 * GAP + 10.0);
const MOVES: [&str; 4] = ["key.forward", "key.left", "key.back", "key.right"];
const CORNERS: [Corner; 4] = [Corner::TopLeft, Corner::TopRight, Corner::BottomLeft, Corner::BottomRight];
const CORNER: u32 = 0;
const CPS: u32 = 1;

const TEXT: [[&str; 8]; 3] = [
    ["Corner", "Top left", "Top right", "Bottom left", "Bottom right", "Clicks per second", "On", "Off"],
    ["Canto", "Superior esquerdo", "Superior direito", "Inferior esquerdo", "Inferior direito", "Cliques por segundo", "Ligado", "Desligado"],
    ["Canto", "Superior esquerdo", "Superior direito", "Inferior esquerdo", "Inferior direito", "Cliques por segundo", "Ligado", "Desligado"],
];

fn lang() -> usize {
    match ui::language().as_str() {
        "pt_br" => 1,
        "pt_pt" => 2,
        _ => 0,
    }
}

struct Keystrokes {
    corner: usize,
    cps: bool,
    last: String,
}

fn down(name: &str) -> (bool, String) {
    input::mapping(name).map_or((false, String::new()), |m| (m.down, m.label))
}

fn key_box(out: &mut Vec<Shape>, [x, y, w, h]: [f32; 4], held: bool, lines: &[&str]) {
    let (fill, ink) = if held { (HELD, INK_HELD) } else { (BACKGROUND, INK) };
    out.push(Shape::Fill(FillShape { rect: Rect { x, y, w, h }, color: fill }));
    let total = lines.len() as f32 * 9.0 - 1.0;
    for (i, line) in lines.iter().enumerate() {
        let at = Point {
            x: (x + (w - render::text_width(line)) / 2.0).round(),
            y: (y + (h - total) / 2.0).round() + i as f32 * 9.0,
        };
        out.push(Shape::Text(TextShape { text: (*line).to_owned(), at, color: ink, centered: false, shadow: !held }));
    }
}

impl Keystrokes {
    fn load() -> Self {
        let mut k = Self { corner: 0, cps: true, last: String::new() };
        if let Some(text) = storage::get(FILE).and_then(|v| String::from_utf8(v).ok()) {
            for line in text.lines() {
                match line.split_once('=') {
                    Some(("corner", v)) => k.corner = v.trim().parse::<usize>().unwrap_or(0).min(3),
                    Some(("cps", v)) => k.cps = v.trim() != "false",
                    _ => {}
                }
            }
        }
        k
    }

    fn save(&self) {
        let _ = storage::set(FILE, format!("corner={}\ncps={}\n", self.corner, self.cps).as_bytes());
    }

    fn page(&self) {
        let t = TEXT[lang()];
        ui::set_settings_v2(&[
            Widget::Button(ButtonWidget { id: CORNER, label: format!("{}: {}", t[0], t[1 + self.corner]) }),
            Widget::Button(ButtonWidget {
                id: CPS,
                label: format!("{}: {}", t[5], if self.cps { t[6] } else { t[7] }),
            }),
        ]);
        ui::set_resets(&[CORNER, CPS]);
    }

    fn draw(&mut self) {
        let (w, _) = SIZE;
        let step = KEY + GAP;
        let mut out = Vec::with_capacity(24);
        let moves = MOVES.map(down);
        let (l, r) = input::clicks_per_second();
        let (attack, use_, jump) = (down("key.attack").0, down("key.use").0, down("key.jump").0);
        let key = format!("{moves:?}{attack}{use_}{jump}{l},{r}{}{}", self.corner, self.cps);
        if key == self.last {
            return;
        }
        self.last = key;
        let [forward, left, back, right] = &moves;
        key_box(&mut out, [step, 0.0, KEY, KEY], forward.0, &[&forward.1]);
        for (i, key) in [left, back, right].into_iter().enumerate() {
            key_box(&mut out, [i as f32 * step, step, KEY, KEY], key.0, &[&key.1]);
        }
        let mw = (w - GAP) / 2.0;
        let my = 2.0 * step;
        for (i, (held, label, cps)) in
            [(attack, "LMB", l), (use_, "RMB", r)].into_iter().enumerate()
        {
            let rate = format!("{cps} CPS");
            let lines: Vec<&str> = if self.cps { vec![label, &rate] } else { vec![label] };
            key_box(&mut out, [i as f32 * (mw + GAP), my, mw, KEY], held, &lines);
        }
        let sy = my + step;
        key_box(&mut out, [0.0, sy, w, 10.0], jump, &[]);
        let bar = if jump { 0xFF14_1414 } else { 0xFFFF_FFFF };
        out.push(Shape::Fill(FillShape { rect: Rect { x: w / 2.0 - 12.0, y: sy + 4.0, w: 24.0, h: 2.0 }, color: bar }));
        render::set_corner(CORNERS[self.corner], SIZE.0, SIZE.1, &out);
    }
}

impl Mod for Keystrokes {
    fn init() -> Result<(Self, Registration), String> {
        let k = Keystrokes::load();
        k.page();
        Ok((k, Registration::new().on_frame()))
    }

    fn frame(&mut self, _seconds: f32) {
        self.draw();
    }

    fn ui_select(&mut self, id: u32, row: u32) {
        match (id, row) {
            (CORNER, u32::MAX) => self.corner = 0,
            (CPS, u32::MAX) => self.cps = true,
            (CORNER, _) => self.corner = (self.corner + 1) % 4,
            (CPS, _) => self.cps = !self.cps,
            _ => return,
        }
        self.last.clear();
        self.save();
        self.page();
    }

    fn ui_settings_opened(&mut self) {
        self.page();
    }
}

horizon_mod_sdk::export_mod!(Keystrokes);
