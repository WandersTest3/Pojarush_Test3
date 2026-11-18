use horizon_mod_sdk::{
    Mod, Registration,
    input::{self, Event},
    ui::{self, ButtonWidget, ListWidget, ModInfo, Screen, TextWidget, Widget, WidgetV2},
};

const KEY: &str = "mod_menu";
const LIST: u32 = 1;
const SETTINGS: u32 = 2;
const OPEN: u32 = 3;
const FILTER: u32 = 4;
const TITLE: u32 = 0xFFFF_FF00;
const GRAY: u32 = 0xAAAAAA;
const PURPLE: u32 = 0xB98CF2;

const LANGS: [(&str, [&str; 16]); 3] = [
    ("en", ["Mods", "Settings", "Open the mod list", "Version", "Authors", "Minecraft", "API", "License", "Website", "Source", "Issues", "Running", "Off", "Show: all mods", "Show: running mods", "No description."]),
    ("pt_br", ["Mods", "Configurações", "Abrir a lista de mods", "Versão", "Autores", "Minecraft", "API", "Licença", "Site", "Código-fonte", "Problemas", "Rodando", "Desligado", "Mostrar: todos os mods", "Mostrar: mods rodando", "Sem descrição."]),
    ("pt_pt", ["Mods", "Definições", "Abrir a lista de mods", "Versão", "Autores", "Minecraft", "API", "Licença", "Site", "Código-fonte", "Problemas", "A correr", "Desligado", "Mostrar: todos os mods", "Mostrar: mods a correr", "Sem descrição."]),
];

fn strings() -> &'static [&'static str; 16] {
    let code = ui::language().to_lowercase();
    let find = |c: &str| LANGS.iter().find(|(l, _)| *l == c).map(|(_, s)| s);
    find(&code)
        .or_else(|| (code.starts_with("pt")).then(|| find("pt_br")).flatten())
        .unwrap_or(&LANGS[0].1)
}

fn text(id: u32, text: String, color: u32) -> Widget {
    Widget::Text(TextWidget { id, text, color })
}

fn wrap(line: &str, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for word in line.split_whitespace() {
        if !cur.is_empty() && cur.chars().count() + 1 + word.chars().count() > width {
            out.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(word);
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

struct Menu {
    mods: Vec<ModInfo>,
    selected: usize,
    running_only: bool,
    open: bool,
}

impl Menu {
    fn shown(&self) -> Vec<&ModInfo> {
        self.mods.iter().filter(|m| m.running || !self.running_only).collect()
    }

    fn refresh(&mut self) {
        let mut mods = ui::mods();
        mods.sort_by_key(|m| m.name.to_lowercase());
        self.mods = mods;
        let n = self.shown().len();
        if self.selected >= n {
            self.selected = n.saturating_sub(1);
        }
    }

    fn screen(&mut self) {
        self.refresh();
        let t = strings();
        let shown = self.shown();
        let rows = shown
            .iter()
            .map(|m| {
                if m.running {
                    format!("{}  {}", m.name, m.version)
                } else {
                    format!("{}  {} ({})", m.name, m.version, t[12])
                }
            })
            .collect();
        let mut widgets = vec![
            Widget::Button(ButtonWidget {
                id: FILTER,
                label: (if self.running_only { t[14] } else { t[13] }).into(),
            }),
            Widget::List(ListWidget {
                id: LIST,
                rows,
                selected: (!shown.is_empty()).then_some(self.selected as u32),
            }),
        ];
        if let Some(m) = shown.get(self.selected) {
            widgets.push(text(10, format!("{} ({})", m.name, m.id), PURPLE));
            let state = if m.running { t[11] } else { t[12] };
            widgets.push(text(11, format!("{}: {}  ·  {}", t[3], m.version, state), 0));
            let description = if m.description.is_empty() { t[15].to_string() } else { m.description.clone() };
            for (i, line) in wrap(&description, 52).into_iter().take(6).enumerate() {
                widgets.push(text(20 + i as u32, line, GRAY));
            }
            let mut info = Vec::new();
            if !m.authors.is_empty() {
                info.push(format!("{}: {}", t[4], m.authors.join(", ")));
            }
            info.push(format!(
                "{}: {}  ·  {}: {}",
                t[5],
                if m.minecraft.is_empty() { "*" } else { &m.minecraft },
                t[6],
                m.api
            ));
            for (label, value) in [(t[7], &m.license), (t[8], &m.homepage), (t[9], &m.sources), (t[10], &m.issues)] {
                if !value.is_empty() {
                    info.push(format!("{label}: {value}"));
                }
            }
            for (i, line) in info.into_iter().enumerate() {
                widgets.push(text(40 + i as u32, line, 0));
            }
            if m.has_settings && m.id != "pojarush_modmenu" {
                widgets.push(Widget::Button(ButtonWidget { id: SETTINGS, label: t[1].into() }));
            }
        }
        ui::open_screen(&Screen {
            title: format!("{} ({})", t[0], shown.len()),
            widgets,
        });
        self.open = true;
    }

    fn page(&self) {
        ui::set_settings_v2(&[WidgetV2::Button(ButtonWidget { id: OPEN, label: strings()[2].into() })]);
    }
}

impl Mod for Menu {
    fn init() -> Result<(Self, Registration), String> {
        input::register_key(KEY, "Mod Menu", "key.keyboard.m")?;
        let menu = Menu { mods: Vec::new(), selected: 0, running_only: false, open: false };
        menu.page();
        ui::set_title_button("Mods");
        Ok((menu, Registration::new().on_frame()))
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

    fn ui_select(&mut self, id: u32, row: u32) {
        match id {
            LIST => {
                self.selected = row as usize;
                self.screen();
            }
            FILTER => {
                self.running_only = !self.running_only;
                self.selected = 0;
                self.screen();
            }
            SETTINGS => {
                let id = self.shown().get(self.selected).map(|m| m.id.clone());
                if let Some(id) = id
                    && ui::open_mod_settings(&id)
                {
                    self.open = false;
                }
            }
            OPEN | TITLE => ui::open_mod_list(),
            _ => {}
        }
    }

    fn ui_closed(&mut self) {
        self.open = false;
    }

    fn ui_settings_opened(&mut self) {
        self.page();
    }
}

horizon_mod_sdk::export_mod!(Menu);
