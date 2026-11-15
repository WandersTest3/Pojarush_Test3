use horizon_mod_sdk::{
    Mod, Registration,
    render::{self, FillShape, Point, Rect, Shape, TextShape},
    storage,
    ui::{self, ButtonWidget, Widget},
};

const SIZE_MIN: u32 = 1;
const SIZE_MAX: u32 = 16;
const POS_STEP: u32 = 5;
const COLORS: [(&str, u32); 5] = [
    ("White", 0xFFFFFF),
    ("Purple", 0xB98CF2),
    ("Green", 0x55FF55),
    ("Yellow", 0xFFFF55),
    ("By FPS", 0),
];
const CORNERS: [(&str, u32, u32); 5] = [
    ("Top left", 0, 0),
    ("Top right", 100, 0),
    ("Bottom left", 0, 100),
    ("Bottom right", 100, 100),
    ("Top center", 50, 0),
];

const SHOW: u32 = 1;
const SIZE_DOWN: u32 = 2;
const SIZE_UP: u32 = 3;
const X_DOWN: u32 = 4;
const X_UP: u32 = 5;
const Y_DOWN: u32 = 6;
const Y_UP: u32 = 7;
const CORNER: u32 = 8;
const LOWS: u32 = 9;
const BACKGROUND: u32 = 10;
const COLOR: u32 = 11;
const RESET: u32 = 12;

struct Fps {
    show: bool,
    size: u32,
    x: u32,
    y: u32,
    lows: bool,
    background: bool,
    color: u32,
    last: (u32, u32, u32, u32, u32),
    drawn: bool,
}

fn on_off(on: bool) -> &'static str {
    if on { "On" } else { "Off" }
}

fn stored(key: &str, default: u32, min: u32, max: u32) -> u32 {
    storage::get(key)
        .and_then(|v| String::from_utf8(v).ok())
        .and_then(|v| v.trim().parse::<u32>().ok())
        .map_or(default, |v| v.clamp(min, max))
}

fn button(id: u32, label: String) -> Widget {
    Widget::Button(ButtonWidget { id, label })
}

impl Fps {
    fn scale(&self) -> f32 {
        self.size as f32 * 0.5
    }

    fn page(&self) {
        let corner = CORNERS
            .iter()
            .find(|c| c.1 == self.x && c.2 == self.y)
            .map_or("Custom", |c| c.0);
        ui::set_settings(&[
            button(SHOW, format!("FPS counter: {}", on_off(self.show))),
            button(SIZE_DOWN, format!("Size: - ({}/{})", self.size, SIZE_MAX)),
            button(SIZE_UP, format!("Size: + ({}/{})", self.size, SIZE_MAX)),
            button(X_DOWN, format!("Horizontal position: - ({}%)", self.x)),
            button(X_UP, format!("Horizontal position: + ({}%)", self.x)),
            button(Y_DOWN, format!("Vertical position: - ({}%)", self.y)),
            button(Y_UP, format!("Vertical position: + ({}%)", self.y)),
            button(CORNER, format!("Place at: {corner}")),
            button(LOWS, format!("Minimum and 1% low: {}", on_off(self.lows))),
            button(BACKGROUND, format!("Background: {}", on_off(self.background))),
            button(COLOR, format!("Color: {}", COLORS[self.color as usize].0)),
            button(RESET, "Reset to defaults".into()),
        ]);
    }

    fn save(&self) {
        let put = |key: &str, value: u32| {
            let _ = storage::set(key, value.to_string().as_bytes());
        };
        put("show", u32::from(self.show));
        put("size", self.size);
        put("x", self.x);
        put("y", self.y);
        put("lows", u32::from(self.lows));
        put("background", u32::from(self.background));
        put("color", self.color);
    }

    fn tint(&self, fps: u32) -> u32 {
        let rgb = COLORS[self.color as usize].1;
        let rgb = if rgb != 0 {
            rgb
        } else if fps >= 60 {
            0x55FF55
        } else if fps >= 30 {
            0xFFFF55
        } else {
            0xFF5555
        };
        rgb
    }

    fn lines(&self, fps: u32, min: u32, low: u32) -> Vec<String> {
        let mut lines = vec![format!("{fps} fps")];
        if self.lows {
            lines.push(format!("min {min}"));
            lines.push(format!("1% low {low}"));
        }
        lines
    }

    fn draw(&mut self) {
        if !self.show {
            if self.drawn {
                render::set_hud(&[]);
                self.drawn = false;
            }
            return;
        }
        let stats = render::frame_stats();
        let (w, h) = render::gui_size();
        let key = (stats.fps, stats.min, stats.low, w as u32, h as u32);
        if self.drawn && key == self.last {
            return;
        }
        self.last = key;
        self.drawn = true;
        let lines = self.lines(stats.fps, stats.min, stats.low);
        let scale = self.scale();
        let (w, h) = (w / scale, h / scale);
        let pad = 2.0;
        let line_h = 10.0;
        let text_w = lines.iter().map(|l| render::text_width(l)).fold(0.0, f32::max);
        let box_w = text_w + pad * 2.0;
        let box_h = lines.len() as f32 * line_h + pad * 2.0 - 1.0;
        let left = (w - box_w).max(0.0) * self.x as f32 / 100.0;
        let top = (h - box_h).max(0.0) * self.y as f32 / 100.0;
        let mut shapes = Vec::new();
        if self.background {
            shapes.push(Shape::Fill(FillShape {
                rect: Rect { x: left, y: top, w: box_w, h: box_h },
                color: 0x80000000,
            }));
        }
        let color = self.tint(stats.fps);
        for (row, line) in lines.iter().enumerate() {
            shapes.push(Shape::Text(TextShape {
                text: line.clone(),
                at: Point { x: left + pad, y: top + pad + row as f32 * line_h },
                color: if row == 0 { color } else { 0xFFFFFF },
                centered: false,
                shadow: true,
            }));
        }
        render::set_hud_scale(scale);
        render::set_hud(&shapes);
    }
}

impl Mod for Fps {
    fn init() -> Result<(Self, Registration), String> {
        let fps = Fps {
            show: stored("show", 1, 0, 1) == 1,
            size: stored("size", 3, SIZE_MIN, SIZE_MAX),
            x: stored("x", 50, 0, 100),
            y: stored("y", 0, 0, 100),
            lows: stored("lows", 0, 0, 1) == 1,
            background: stored("background", 1, 0, 1) == 1,
            color: stored("color", 0, 0, COLORS.len() as u32 - 1),
            last: (0, 0, 0, 0, 0),
            drawn: false,
        };
        fps.page();
        Ok((fps, Registration::new().on_frame()))
    }

    fn frame(&mut self, _seconds: f32) {
        self.draw();
    }

    fn ui_select(&mut self, id: u32, _row: u32) {
        match id {
            SHOW => self.show = !self.show,
            SIZE_DOWN => self.size = self.size.saturating_sub(1).max(SIZE_MIN),
            SIZE_UP => self.size = (self.size + 1).min(SIZE_MAX),
            X_DOWN => self.x = self.x.saturating_sub(POS_STEP),
            X_UP => self.x = (self.x + POS_STEP).min(100),
            Y_DOWN => self.y = self.y.saturating_sub(POS_STEP),
            Y_UP => self.y = (self.y + POS_STEP).min(100),
            CORNER => {
                let next = CORNERS
                    .iter()
                    .position(|c| c.1 == self.x && c.2 == self.y)
                    .map_or(0, |i| (i + 1) % CORNERS.len());
                self.x = CORNERS[next].1;
                self.y = CORNERS[next].2;
            }
            LOWS => self.lows = !self.lows,
            BACKGROUND => self.background = !self.background,
            COLOR => self.color = (self.color + 1) % COLORS.len() as u32,
            RESET => {
                self.show = true;
                self.size = 3;
                self.x = 50;
                self.y = 0;
                self.lows = false;
                self.background = true;
                self.color = 0;
            }
            _ => return,
        }
        self.drawn = false;
        self.last = (0, 0, 0, 0, 0);
        self.save();
        self.page();
    }

    fn ui_settings_opened(&mut self) {
        self.page();
    }
}

horizon_mod_sdk::export_mod!(Fps);
