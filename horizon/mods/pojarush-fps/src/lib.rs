use horizon_mod_sdk::{
    Mod, Registration,
    render::{self, FillShape, Point, Rect, Shape, TextShape},
    storage,
    ui::{self, ButtonWidget, SliderWidget, Widget},
};

const COLORS: [(&str, u32); 5] = [
    ("White", 0xFFFFFF),
    ("Purple", 0xB98CF2),
    ("Green", 0x55FF55),
    ("Yellow", 0xFFFF55),
    ("By FPS", 0),
];
const CORNERS: [(&str, u32, u32); 5] = [
    ("Top left", 0, 0),
    ("Top right", 1000, 0),
    ("Bottom left", 0, 1000),
    ("Bottom right", 1000, 1000),
    ("Top center", 500, 0),
];

const SCALE_MIN: u32 = 25;
const SCALE_MAX: u32 = 1600;
const SCALE_STEP: u32 = 5;
const OFFSET_RANGE: i32 = 500;
const PAD_MAX: u32 = 32;
const SHOW: u32 = 1;
const CORNER: u32 = 8;
const LOWS: u32 = 9;
const BACKGROUND: u32 = 10;
const COLOR: u32 = 11;
const RESET: u32 = 12;
const S_SIZE: u32 = 20;
const S_X: u32 = 21;
const S_Y: u32 = 22;
const S_XOFF: u32 = 23;
const S_YOFF: u32 = 24;
const S_PAD: u32 = 25;
const S_OPACITY: u32 = 26;

struct Fps {
    show: bool,
    scale: u32,
    x: u32,
    y: u32,
    x_off: i32,
    y_off: i32,
    pad: u32,
    opacity: u32,
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

fn stored_i(key: &str, default: i32) -> i32 {
    storage::get(key)
        .and_then(|v| String::from_utf8(v).ok())
        .and_then(|v| v.trim().parse::<i32>().ok())
        .map_or(default, |v| v.clamp(-OFFSET_RANGE, OFFSET_RANGE))
}

fn button(id: u32, label: String) -> Widget {
    Widget::Button(ButtonWidget { id, label })
}

fn slider(id: u32, label: String, steps: u32, position: u32) -> Widget {
    Widget::Slider(SliderWidget { id, label, steps, position })
}

impl Fps {
    fn scale(&self) -> f32 {
        self.scale as f32 / 100.0
    }

    fn slider_label(&self, id: u32) -> (String, u32, u32) {
        match id {
            S_SIZE => (
                format!("Size: {:.2}x", self.scale()),
                (SCALE_MAX - SCALE_MIN) / SCALE_STEP,
                (self.scale - SCALE_MIN) / SCALE_STEP,
            ),
            S_X => (format!("Horizontal position: {:.1}%", self.x as f32 / 10.0), 1000, self.x),
            S_Y => (format!("Vertical position: {:.1}%", self.y as f32 / 10.0), 1000, self.y),
            S_XOFF => (
                format!("Horizontal offset: {} px", self.x_off),
                OFFSET_RANGE as u32 * 2,
                (self.x_off + OFFSET_RANGE) as u32,
            ),
            S_YOFF => (
                format!("Vertical offset: {} px", self.y_off),
                OFFSET_RANGE as u32 * 2,
                (self.y_off + OFFSET_RANGE) as u32,
            ),
            S_PAD => (format!("Background padding: {:.1}", self.pad as f32 / 2.0), PAD_MAX, self.pad),
            _ => (
                format!("Background opacity: {}%", self.opacity),
                100,
                self.opacity,
            ),
        }
    }

    fn slider_widget(&self, id: u32) -> Widget {
        let (label, steps, position) = self.slider_label(id);
        slider(id, label, steps, position)
    }

    fn page(&self) {
        ui::set_settings(&[
            button(SHOW, format!("FPS counter: {}", on_off(self.show))),
            button(LOWS, format!("Minimum and 1% low: {}", on_off(self.lows))),
            button(BACKGROUND, format!("Background: {}", on_off(self.background))),
            button(COLOR, format!("Color: {}", COLORS[self.color as usize].0)),
            button(CORNER, "Move to the next corner".into()),
            self.slider_widget(S_SIZE),
            self.slider_widget(S_X),
            self.slider_widget(S_Y),
            self.slider_widget(S_XOFF),
            self.slider_widget(S_YOFF),
            self.slider_widget(S_PAD),
            self.slider_widget(S_OPACITY),
            button(RESET, "Reset to defaults".into()),
        ]);
    }

    fn save(&self) {
        let put = |key: &str, value: i64| {
            let _ = storage::set(key, value.to_string().as_bytes());
        };
        put("show", i64::from(self.show));
        put("scale", i64::from(self.scale));
        put("xt", i64::from(self.x));
        put("yt", i64::from(self.y));
        put("xoff", i64::from(self.x_off));
        put("yoff", i64::from(self.y_off));
        put("pad", i64::from(self.pad));
        put("opacity", i64::from(self.opacity));
        put("lows", i64::from(self.lows));
        put("background", i64::from(self.background));
        put("color", i64::from(self.color));
    }

    fn defaults(&mut self) {
        self.show = true;
        self.scale = 150;
        self.x = 500;
        self.y = 0;
        self.x_off = 0;
        self.y_off = 0;
        self.pad = 4;
        self.opacity = 50;
        self.lows = false;
        self.background = true;
        self.color = 0;
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
        let pad = self.pad as f32 / 2.0;
        let line_h = 10.0;
        let text_w = lines.iter().map(|l| render::text_width(l)).fold(0.0, f32::max);
        let box_w = text_w + pad * 2.0;
        let box_h = lines.len() as f32 * line_h + pad * 2.0 - 1.0;
        let left = (w - box_w).max(0.0) * self.x as f32 / 1000.0 + self.x_off as f32 / scale;
        let top = (h - box_h).max(0.0) * self.y as f32 / 1000.0 + self.y_off as f32 / scale;
        let mut shapes = Vec::new();
        if self.background {
            shapes.push(Shape::Fill(FillShape {
                rect: Rect { x: left, y: top, w: box_w, h: box_h },
                color: (self.opacity * 255 / 100) << 24,
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
        let old_scale = stored("size", 3, 1, 16) * 50;
        let mut fps = Fps {
            show: stored("show", 1, 0, 1) == 1,
            scale: stored("scale", old_scale, SCALE_MIN, SCALE_MAX) / SCALE_STEP * SCALE_STEP,
            x: stored("xt", stored("x", 50, 0, 100) * 10, 0, 1000),
            y: stored("yt", stored("y", 0, 0, 100) * 10, 0, 1000),
            x_off: stored_i("xoff", 0),
            y_off: stored_i("yoff", 0),
            pad: stored("pad", 4, 0, PAD_MAX),
            opacity: stored("opacity", 50, 0, 100),
            lows: stored("lows", 0, 0, 1) == 1,
            background: stored("background", 1, 0, 1) == 1,
            color: stored("color", 0, 0, COLORS.len() as u32 - 1),
            last: (0, 0, 0, 0, 0),
            drawn: false,
        };
        fps.scale = fps.scale.max(SCALE_MIN);
        fps.page();
        Ok((fps, Registration::new().on_frame()))
    }

    fn frame(&mut self, _seconds: f32) {
        self.draw();
    }

    fn ui_select(&mut self, id: u32, row: u32) {
        let slid = match id {
            S_SIZE => {
                self.scale = SCALE_MIN + row * SCALE_STEP;
                true
            }
            S_X => {
                self.x = row.min(1000);
                true
            }
            S_Y => {
                self.y = row.min(1000);
                true
            }
            S_XOFF => {
                self.x_off = row as i32 - OFFSET_RANGE;
                true
            }
            S_YOFF => {
                self.y_off = row as i32 - OFFSET_RANGE;
                true
            }
            S_PAD => {
                self.pad = row.min(PAD_MAX);
                true
            }
            S_OPACITY => {
                self.opacity = row.min(100);
                true
            }
            _ => false,
        };
        if slid {
            let (label, _, position) = self.slider_label(id);
            ui::set_slider(id, position, &label);
            self.drawn = false;
            self.last = (0, 0, 0, 0, 0);
            self.save();
            return;
        }
        match id {
            SHOW => self.show = !self.show,
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
            RESET => self.defaults(),
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
