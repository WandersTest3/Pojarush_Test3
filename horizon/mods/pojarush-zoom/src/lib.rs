use horizon_mod_sdk::{
    Mod, Registration,
    input::{self, Event},
    render::{self, Zoom as ZoomView},
};

const KEY: &str = "zoom";
const FACTOR: f32 = 4.0;

struct Zoom {
    on: bool,
}

impl Zoom {
    fn show(&self) {
        let factor = if self.on { FACTOR } else { 1.0 };
        render::set_zoom(ZoomView {
            factor,
            smooth: true,
            turn: 1.0 / factor,
        });
    }
}

impl Mod for Zoom {
    fn init() -> Result<(Self, Registration), String> {
        input::register_key(KEY, "Zoom", "key.keyboard.c")?;
        Ok((Zoom { on: false }, Registration::new().on_frame()))
    }
    fn frame(&mut self, _seconds: f32) {
        let mut changed = false;
        for event in input::poll() {
            if let Event::KeyDown(k) = event
                && k == KEY
            {
                self.on = !self.on;
                changed = true;
            }
        }
        if changed {
            self.show();
        }
    }
}

horizon_mod_sdk::export_mod!(Zoom);
