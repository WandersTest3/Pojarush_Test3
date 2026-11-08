use horizon_mod_sdk::{
    Mod, Registration,
    render::{self, Corner, FillShape, Point, Rect, Shape, TextShape},
    storage,
    ui::{self, ButtonWidget, Widget},
};
const BACKGROUND: u32 = 0x7800_0000;
const CORNERS: [(Corner, &str, &str); 4] = [
    (Corner::TopLeft, "top_left", "Top left"),
    (Corner::TopRight, "top_right", "Top right"),
    (Corner::BottomLeft, "bottom_left", "Bottom left"),
    (Corner::BottomRight, "bottom_right", "Bottom right"),
];
const CORNER_BUTTON: u32 = 1;
const LOWS_BUTTON: u32 = 2;

struct Fps {
    corner: usize,
    lows: bool,
    shown: String,
}

impl Fps {
    fn text(&self, f: render::FrameRate) -> String {
        if self.lows {
            format!("{} fps  min {}  1% {}", f.fps, f.min, f.low)
        } else {
            format!("{} fps", f.fps)
        }
    }
    fn draw(&mut self) {
        let text = self.text(render::frame_stats());
        if text == self.shown {
            return;
        }
        let w = render::text_width(&text) + 4.0;
        render::set_corner(
            CORNERS[self.corner].0,
            w,
            12.0,
            &[
                Shape::Fill(FillShape {
                    rect: Rect {
                        x: 0.0,
                        y: 0.0,
                        w,
                        h: 12.0,
                    },
                    color: BACKGROUND,
                }),
                Shape::Text(TextShape {
                    text: text.clone(),
                    at: Point { x: 2.0, y: 2.0 },
                    color: 0xFF_FFFF,
                    centered: false,
                    shadow: true,
                }),
            ],
        );
        self.shown = text;
    }
    fn page(&self) {
        let on = |b: bool| if b { "ON" } else { "OFF" };
        ui::set_settings(&[
            Widget::Button(ButtonWidget {
                id: CORNER_BUTTON,
                label: format!("Corner: {}", CORNERS[self.corner].2),
            }),
            Widget::Button(ButtonWidget {
                id: LOWS_BUTTON,
                label: format!("Minimum and 1% low: {}", on(self.lows)),
            }),
        ]);
    }
    fn save(&self) {
        let _ = storage::set("corner", CORNERS[self.corner].1.as_bytes());
        let _ = storage::set("lows", if self.lows { b"true" } else { b"false" });
    }
}

fn stored(key: &str) -> Option<String> {
    storage::get(key).and_then(|v| String::from_utf8(v).ok())
}

impl Mod for Fps {
    fn init() -> Result<(Self, Registration), String> {
        let corner = stored("corner")
            .and_then(|c| CORNERS.iter().position(|k| k.1 == c.trim()))
            .unwrap_or(0);
        let lows = stored("lows").is_some_and(|v| v.trim() == "true");
        let fps = Fps {
            corner,
            lows,
            shown: String::new(),
        };
        fps.page();
        Ok((fps, Registration::new().on_tick()))
    }
    fn tick(&mut self, _tick: u64) {
        self.draw();
    }
    fn ui_select(&mut self, id: u32, _row: u32) {
        match id {
            CORNER_BUTTON => self.corner = (self.corner + 1) % CORNERS.len(),
            LOWS_BUTTON => self.lows = !self.lows,
            _ => return,
        }
        self.save();
        self.page();
        self.shown.clear();
        self.draw();
    }
    fn ui_settings_opened(&mut self) {
        self.page();
    }
}

horizon_mod_sdk::export_mod!(Fps);
