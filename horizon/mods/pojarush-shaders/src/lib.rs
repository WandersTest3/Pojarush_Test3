use horizon_mod_sdk::{
    Mod, Registration,
    input::{self, Event},
    storage,
    ui::{self, ButtonWidget, ListWidget, Screen, TextWidget, Widget, WidgetV2},
};

const MENU: &str = "shader_menu";
const TOGGLE: &str = "shader_toggle";
const RELOAD: &str = "shader_reload";
const LIST: u32 = 1;
const ON_OFF: u32 = 2;
const OPTIONS: u32 = 3;
const RELOAD_BUTTON: u32 = 4;
const FOLDER: u32 = 5;
const OPEN: u32 = 6;
const FILE: &str = "pojarush_shaders.txt";
const VIDEO: u32 = 0xFFFF_FF01;

const LANGS: [(&str, [&str; 12]); 3] = [
    ("en", ["Shader Packs", "Shaders: On", "Shaders: Off", "Shader Pack Settings...", "Reload", "Options > Shader Packs...", "Open the shader pack list", "In use", "No shader packs in the shaderpacks folder.", "Shaders off", "Shaders on", "Shader Packs..."]),
    ("pt_br", ["Pacotes de shaders", "Shaders: ligados", "Shaders: desligados", "Configurações do shader...", "Recarregar", "Opções > Pacotes de shaders...", "Abrir a lista de shaders", "Em uso", "Nenhum shader na pasta shaderpacks.", "Shaders desligados", "Shaders ligados", "Pacotes de shaders..."]),
    ("pt_pt", ["Pacotes de shaders", "Shaders: ligados", "Shaders: desligados", "Definições do shader...", "Recarregar", "Opções > Pacotes de shaders...", "Abrir a lista de shaders", "Em uso", "Nenhum shader na pasta shaderpacks.", "Shaders desligados", "Shaders ligados", "Pacotes de shaders..."]),
];

fn strings() -> &'static [&'static str; 12] {
    let code = ui::language().to_lowercase();
    let find = |c: &str| LANGS.iter().find(|(l, _)| *l == c).map(|(_, s)| s);
    find(&code)
        .or_else(|| (code.starts_with("pt")).then(|| find("pt_br")).flatten())
        .unwrap_or(&LANGS[0].1)
}

struct Shaders {
    last: String,
    packs: Vec<String>,
    open: bool,
}

impl Shaders {
    fn current(&self) -> String {
        ui::shader_pack()
    }

    fn remember(&mut self, name: &str) {
        if !name.is_empty() && self.last != name {
            self.last = name.to_owned();
            let _ = storage::set(FILE, format!("last={name}\n").as_bytes());
        }
    }

    fn toggle(&mut self) {
        let current = self.current();
        if current.is_empty() {
            let last = if self.last.is_empty() {
                ui::shader_packs().into_iter().next().unwrap_or_default()
            } else {
                self.last.clone()
            };
            if !last.is_empty() {
                self.remember(&last);
                ui::set_shader_pack(&last);
            }
        } else {
            self.remember(&current);
            ui::set_shader_pack("");
        }
    }

    fn screen(&mut self, picked: Option<&str>) {
        let t = strings();
        self.packs = ui::shader_packs();
        let current = picked.map_or_else(|| self.current(), str::to_owned);
        let on = !current.is_empty();
        let mut widgets = vec![Widget::Button(ButtonWidget {
            id: ON_OFF,
            label: (if on { t[1] } else { t[2] }).into(),
        })];
        if self.packs.is_empty() {
            widgets.push(Widget::Text(TextWidget { id: 10, text: t[8].into(), color: 0xAAAAAA }));
        } else {
            widgets.push(Widget::List(ListWidget {
                id: LIST,
                rows: self.packs.clone(),
                selected: self.packs.iter().position(|p| *p == current).map(|i| i as u32),
            }));
        }
        if on {
            widgets.push(Widget::Text(TextWidget {
                id: 11,
                text: format!("{}: {current}", t[7]),
                color: 0xB98CF2,
            }));
            widgets.push(Widget::Button(ButtonWidget { id: OPTIONS, label: t[3].into() }));
            widgets.push(Widget::Button(ButtonWidget { id: RELOAD_BUTTON, label: t[4].into() }));
        }
        widgets.push(Widget::Button(ButtonWidget { id: FOLDER, label: t[5].into() }));
        ui::open_screen(&Screen { title: t[0].into(), widgets });
        self.open = true;
    }

    fn page(&self) {
        ui::set_settings_v2(&[WidgetV2::Button(ButtonWidget { id: OPEN, label: strings()[6].into() })]);
    }
}

impl Mod for Shaders {
    fn init() -> Result<(Self, Registration), String> {
        let pt = ui::language().starts_with("pt_");
        input::register_key(MENU, if pt { "Lista de pacotes de shaders" } else { "Shader pack list" }, "key.keyboard.o")?;
        input::register_key(TOGGLE, if pt { "Ligar/desligar shaders" } else { "Toggle shaders" }, "key.keyboard.k")?;
        input::register_key(RELOAD, if pt { "Recarregar shaders" } else { "Reload shaders" }, "key.keyboard.r")?;
        let last = storage::get(FILE)
            .and_then(|v| String::from_utf8(v).ok())
            .and_then(|t| t.lines().find_map(|l| l.strip_prefix("last=").map(str::to_owned)))
            .unwrap_or_default();
        let shaders = Shaders { last, packs: Vec::new(), open: false };
        shaders.page();
        ui::set_video_button(strings()[11]);
        Ok((shaders, Registration::new().on_frame()))
    }

    fn frame(&mut self, _seconds: f32) {
        for event in input::poll() {
            match event {
                Event::KeyDown(k) if k == MENU => ui::open_shader_packs(),
                Event::KeyDown(k) if k == TOGGLE => self.toggle(),
                Event::KeyDown(k) if k == RELOAD => ui::reload_shader_pack(),
                _ => {}
            }
        }
    }

    fn ui_select(&mut self, id: u32, row: u32) {
        match id {
            LIST => {
                if let Some(name) = self.packs.get(row as usize).cloned() {
                    self.remember(&name);
                    ui::set_shader_pack(&name);
                    self.screen(Some(&name));
                }
            }
            ON_OFF => {
                let was = self.current();
                self.toggle();
                let next = if was.is_empty() { self.last.clone() } else { String::new() };
                self.screen(Some(&next));
            }
            OPTIONS => {
                if ui::open_shader_options() {
                    self.open = false;
                }
            }
            RELOAD_BUTTON => ui::reload_shader_pack(),
            FOLDER => {
                ui::open_shader_packs();
                self.open = false;
            }
            OPEN => ui::open_shader_packs(),
            VIDEO => ui::open_shader_packs(),
            _ => {}
        }
    }

    fn ui_closed(&mut self) {
        self.open = false;
    }

    fn ui_settings_opened(&mut self) {
        self.page();
        ui::open_shader_packs();
    }
}

horizon_mod_sdk::export_mod!(Shaders);
