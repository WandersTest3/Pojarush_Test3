use horizon_mod_sdk::{
    Mod, Registration,
    input::{self, Event},
    ui,
};

const KEY: &str = "mod_menu";
const TITLE: u32 = 0xFFFF_FF00;

fn key_label() -> &'static str {
    match ui::language().as_str() {
        "pt_br" | "pt_pt" => "Abrir a lista de mods",
        _ => "Open the mod list",
    }
}

struct Menu;

impl Mod for Menu {
    fn init() -> Result<(Self, Registration), String> {
        input::register_key(KEY, key_label(), "key.keyboard.m")?;
        ui::set_title_button("Mods");
        ui::set_settings_v2(&[]);
        Ok((Menu, Registration::new().on_frame()))
    }

    fn frame(&mut self, _seconds: f32) {
        for event in input::poll() {
            if let Event::KeyDown(k) = event
                && k == KEY
            {
                ui::open_mod_list();
            }
        }
    }

    fn ui_select(&mut self, id: u32, _row: u32) {
        if id == TITLE {
            ui::open_mod_list();
        }
    }
}

horizon_mod_sdk::export_mod!(Menu);
