use horizon_mod_sdk::{
    Mod, Registration, game,
    input::{self, Event},
    render::{self, ModelPlace},
};

const KEY: &str = "place_model";

struct Demo {
    model: Option<u32>,
}

impl Mod for Demo {
    fn init() -> Result<(Self, Registration), String> {
        input::register_key(KEY, if horizon_mod_sdk::ui::language().starts_with("pt_") { "Pôr o modelo de demonstração" } else { "Place the demo model" }, "key.keyboard.j")?;
        Ok((Demo { model: None }, Registration::new().on_frame()))
    }

    fn frame(&mut self, _seconds: f32) {
        for event in input::poll() {
            let Event::KeyDown(k) = event else { continue };
            if k != KEY {
                continue;
            }
            if let Some(id) = self.model.take() {
                render::remove_model(id);
                continue;
            }
            let Some(p) = game::get_player() else { continue };
            let yaw = p.yaw.to_radians();
            let place = ModelPlace {
                x: p.x - f64::from(yaw.sin()) * 3.0,
                y: p.y,
                z: p.z + f64::from(yaw.cos()) * 3.0,
                yaw: p.yaw + 180.0,
                pitch: 0.0,
                scale: 1.0,
            };
            if let Ok(id) = render::spawn_model("totem.geo.json", "totem.png", place) {
                render::play_animation(id, "totem.animation.json", "animation.totem.spin", true);
                self.model = Some(id);
            }
        }
    }
}

horizon_mod_sdk::export_mod!(Demo);
