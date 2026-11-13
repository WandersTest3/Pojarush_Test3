use horizon_mod_sdk::{
    Mod, Registration,
    input::{self, Event},
    render::{self, Zoom as ZoomView},
    storage,
    ui::{self, ButtonWidget, Widget},
};

const KEY: &str = "zoom";
const MIN: f32 = 0.25;
const MAX: f32 = 50.0;
const LEVEL_MIN: u32 = 2;
const LEVEL_MAX: u32 = 30;
const SPEED_MIN: u32 = 1;
const SPEED_MAX: u32 = 10;
const STEPS: [f32; 4] = [1.10, 1.25, 1.50, 2.00];
const LEVEL_DOWN: u32 = 1;
const LEVEL_UP: u32 = 2;
const SPEED_DOWN: u32 = 3;
const SPEED_UP: u32 = 4;
const WHEEL_STEP: u32 = 5;
const MODE: u32 = 6;
const KEEP: u32 = 7;
const STEADY: u32 = 8;
const RESET: u32 = 9;

struct Zoom {
    level: u32,
    speed: u32,
    step: u32,
    toggle: bool,
    keep: bool,
    steady: bool,
    wheel: f32,
    active: bool,
    target: f32,
    current: f32,
}

fn on_off(on: bool) -> &'static str {
    if on { "On" } else { "Off" }
}

impl Zoom {
    fn show(&self) {
        render::set_zoom(ZoomView {
            factor: self.current,
            smooth: false,
            turn: if self.steady {
                (1.0 / self.current).min(1.0)
            } else {
                1.0
            },
        });
    }

    fn button(id: u32, label: String) -> Widget {
        Widget::Button(ButtonWidget { id, label })
    }

    fn page(&self) {
        ui::set_settings(&[
            Self::button(LEVEL_DOWN, format!("Zoom distance: - ({}x)", self.level)),
            Self::button(LEVEL_UP, format!("Zoom distance: + ({}x)", self.level)),
            Self::button(
                SPEED_DOWN,
                format!("Animation speed: - ({}/{})", self.speed, SPEED_MAX),
            ),
            Self::button(
                SPEED_UP,
                format!("Animation speed: + ({}/{})", self.speed, SPEED_MAX),
            ),
            Self::button(
                WHEEL_STEP,
                format!("Wheel step: {:.2}x", STEPS[self.step as usize]),
            ),
            Self::button(
                MODE,
                format!("Zoom key: {}", if self.toggle { "Toggle" } else { "Hold" }),
            ),
            Self::button(KEEP, format!("Remember wheel zoom: {}", on_off(self.keep))),
            Self::button(
                STEADY,
                format!("Slower camera while zoomed: {}", on_off(self.steady)),
            ),
            Self::button(RESET, "Reset to defaults".into()),
        ]);
    }

    fn save(&self) {
        let put = |key: &str, value: u32| {
            let _ = storage::set(key, value.to_string().as_bytes());
        };
        put("level", self.level);
        put("speed", self.speed);
        put("step", self.step);
        put("toggle", u32::from(self.toggle));
        put("keep", u32::from(self.keep));
        put("steady", u32::from(self.steady));
    }

    fn rate(&self) -> f32 {
        2.4 * self.speed as f32
    }

    fn start(&mut self) {
        self.active = true;
        let base = self.level as f32;
        self.target = if self.keep {
            (base * self.wheel).clamp(MIN, MAX)
        } else {
            base
        };
        input::claim_wheel(true);
    }

    fn stop(&mut self) {
        self.active = false;
        self.target = 1.0;
        input::claim_wheel(false);
    }

    fn scroll(&mut self, steps: f32) {
        let next = (self.target * STEPS[self.step as usize].powf(steps)).clamp(MIN, MAX);
        self.target = next;
        self.wheel = next / self.level as f32;
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
            step: stored("step", 1, 0, STEPS.len() as u32 - 1),
            toggle: stored("toggle", 0, 0, 1) == 1,
            keep: stored("keep", 0, 0, 1) == 1,
            steady: stored("steady", 1, 0, 1) == 1,
            wheel: 1.0,
            active: false,
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
                    if self.toggle && self.active {
                        self.stop();
                    } else if !self.active {
                        self.start();
                    }
                }
                Event::KeyUp(k) if k == KEY => {
                    if !self.toggle && self.active {
                        self.stop();
                    }
                }
                Event::Wheel(steps) if self.active => self.scroll(steps),
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
        self.current = if (next - self.target).abs() < 0.005 * self.target.max(1.0) {
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
            WHEEL_STEP => self.step = (self.step + 1) % STEPS.len() as u32,
            MODE => {
                self.toggle = !self.toggle;
                if self.active {
                    self.stop();
                }
            }
            KEEP => {
                self.keep = !self.keep;
                self.wheel = 1.0;
            }
            STEADY => self.steady = !self.steady,
            RESET => {
                self.level = 5;
                self.speed = 8;
                self.step = 1;
                self.toggle = false;
                self.keep = false;
                self.steady = true;
                self.wheel = 1.0;
                if self.active {
                    self.stop();
                }
            }
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
