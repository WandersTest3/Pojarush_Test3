use horizon_mod_sdk::{
    Mod, Registration,
    input::{self, Event},
    render::{self, Zoom as ZoomView},
    storage,
    ui::{self, ButtonWidget, Widget},
};

const KEY: &str = "zoom";
const MIN: f32 = 1.25;
const MAX: f32 = 10.0;
const STEP: f32 = 1.25;
const LEVEL_MIN: u32 = 2;
const LEVEL_MAX: u32 = 10;
const SPEED_MIN: u32 = 1;
const SPEED_MAX: u32 = 10;
const LEVEL_DOWN: u32 = 1;
const LEVEL_UP: u32 = 2;
const SPEED_DOWN: u32 = 3;
const SPEED_UP: u32 = 4;

struct Zoom {
    level: u32,
    speed: u32,
    held: bool,
    target: f32,
    current: f32,
}

impl Zoom {
    fn show(&self) {
        render::set_zoom(ZoomView {
            factor: self.current,
            smooth: false,
            turn: 1.0 / self.current,
        });
    }

    fn page(&self) {
        ui::set_settings(&[
            Widget::Button(ButtonWidget {
                id: LEVEL_DOWN,
                label: format!("Zoom distance: - ({}x)", self.level),
            }),
            Widget::Button(ButtonWidget {
                id: LEVEL_UP,
                label: format!("Zoom distance: + ({}x)", self.level),
            }),
            Widget::Button(ButtonWidget {
                id: SPEED_DOWN,
                label: format!("Animation speed: - ({}/{})", self.speed, SPEED_MAX),
            }),
            Widget::Button(ButtonWidget {
                id: SPEED_UP,
                label: format!("Animation speed: + ({}/{})", self.speed, SPEED_MAX),
            }),
        ]);
    }

    fn save(&self) {
        let _ = storage::set("level", self.level.to_string().as_bytes());
        let _ = storage::set("speed", self.speed.to_string().as_bytes());
    }

    fn rate(&self) -> f32 {
        2.4 * self.speed as f32
    }
}

fn stored(key: &str, default: u32, min: u32, max: u32) -> u32 {
    storage::get(key)
        .and_then(|v| String::from_utf8(v).ok())
        .and_then(|v| v.trim().parse::<u32>().ok())
        .map_or(default, |v| v.clamp(min, max))
}

impl Mod for Zoom {
    fn init() -> Result<(Self, Registration), String> {
        input::register_key(KEY, "Zoom", "key.keyboard.c")?;
        let zoom = Zoom {
            level: stored("level", 5, LEVEL_MIN, LEVEL_MAX),
            speed: stored("speed", 8, SPEED_MIN, SPEED_MAX),
            held: false,
            target: 1.0,
            current: 1.0,
        };
        zoom.page();
        Ok((zoom, Registration::new().on_frame()))
    }

    fn frame(&mut self, seconds: f32) {
        for event in input::poll() {
            match event {
                Event::KeyDown(k) if k == KEY => {
                    self.held = true;
                    self.target = self.level as f32;
                    input::claim_wheel(true);
                }
                Event::KeyUp(k) if k == KEY => {
                    self.held = false;
                    self.target = 1.0;
                    input::claim_wheel(false);
                }
                Event::Wheel(steps) if self.held => {
                    self.target = (self.target * STEP.powf(steps)).clamp(MIN, MAX);
                }
                _ => {}
            }
        }
        if (self.current - self.target).abs() < 0.001 {
            if self.current != self.target {
                self.current = self.target;
                self.show();
            }
            return;
        }
        let k = 1.0 - (-self.rate() * seconds).exp();
        let next = (self.current.ln() + (self.target.ln() - self.current.ln()) * k).exp();
        self.current = if (next - self.target).abs() < 0.005 {
            self.target
        } else {
            next
        };
        self.show();
    }

    fn ui_select(&mut self, id: u32, _row: u32) {
        match id {
            LEVEL_DOWN => self.level = self.level.saturating_sub(1).max(LEVEL_MIN),
            LEVEL_UP => self.level = (self.level + 1).min(LEVEL_MAX),
            SPEED_DOWN => self.speed = self.speed.saturating_sub(1).max(SPEED_MIN),
            SPEED_UP => self.speed = (self.speed + 1).min(SPEED_MAX),
            _ => return,
        }
        self.save();
        self.page();
    }

    fn ui_settings_opened(&mut self) {
        self.page();
    }
}

horizon_mod_sdk::export_mod!(Zoom);
