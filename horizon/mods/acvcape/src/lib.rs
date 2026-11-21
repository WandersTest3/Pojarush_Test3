mod capes;

use capes::CAPES;
use horizon_mod_sdk::{
    Mod, Registration,
    input::{self, Event},
    render, storage,
    ui::{self, ButtonWidget, EntityWidget, ScreenV3, TextWidget, WidgetV3 as Widget},
};
use std::fmt::Write;

const FILE: &str = "acvcape.txt";
const KEY: &str = "open_menu";
const TABS: u32 = 10;
const NONE: u32 = 20;
const FAVORITE: u32 = 21;
const CAPE: u32 = 100;
const MAX_WIDGETS: usize = 64;

fn strings() -> [&'static str; 13] {
    match ui::language().as_str() {
        "pt_br" => ["Acv Cape", "Abrir Menu de Capas", "Todas", "Java", "Bedrock", "Dungeons II", "Favoritos", "Sem Capa", "Favoritar", "Desfavoritar", "Capa atual", "Nenhum favorito ainda.", "Nenhuma capa encontrada."],
        "pt_pt" => ["Acv Cape", "Abrir Menu de Capas", "Todas", "Java", "Bedrock", "Dungeons II", "Favoritos", "Sem Capa", "Favoritar", "Desfavoritar", "Capa atual", "Ainda não há favoritos.", "Não foram encontradas capas."],
        _ => ["Acv Cape", "Open Cape Menu", "All", "Java", "Bedrock", "Dungeons II", "Favorites", "No Cape", "Favorite", "Unfavorite", "Current cape", "No favorites yet.", "No capes found."],
    }
}

struct Acv {
    cape: Option<usize>,
    tab: u32,
    favorites: Vec<usize>,
    shown: Vec<usize>,
    screen: bool,
}

impl Acv {
    fn wear(&self) {
        let file = self.cape.map(|i| format!("capes/{}.png", CAPES[i].0)).unwrap_or_default();
        let _ = render::set_player_cape(&file);
    }

    fn save(&self) {
        let mut text = String::new();
        if let Some(i) = self.cape {
            let _ = writeln!(text, "cape={}", CAPES[i].0);
        }
        let _ = writeln!(text, "tab={}", self.tab);
        let favs: Vec<&str> = self.favorites.iter().map(|&i| CAPES[i].0).collect();
        let _ = writeln!(text, "favorites={}", favs.join(","));
        let _ = storage::set(FILE, text.as_bytes());
    }

    fn widgets(&mut self) -> Vec<Widget> {
        let t = strings();
        self.shown = (0..CAPES.len())
            .filter(|&i| match self.tab {
                0 => true,
                1 => CAPES[i].2 & 1 != 0,
                2 => CAPES[i].2 & 2 != 0,
                3 => CAPES[i].2 & 4 != 0,
                _ => self.favorites.contains(&i),
            })
            .collect();
        let mut w = Vec::new();
        w.push(Widget::Entity(EntityWidget { id: 0, entity: "player:back".into(), height: 76, follow_cursor: true }));
        for (k, label) in [t[2], t[3], t[4], t[5], t[6]].iter().enumerate() {
            let mark = if self.tab == k as u32 { "> " } else { "" };
            w.push(Widget::Button(ButtonWidget { id: TABS + k as u32, label: format!("{mark}{label}") }));
        }
        w.push(Widget::Button(ButtonWidget { id: NONE, label: t[7].into() }));
        if let Some(c) = self.cape {
            let label = if self.favorites.contains(&c) { t[9] } else { t[8] };
            w.push(Widget::Button(ButtonWidget { id: FAVORITE, label: label.into() }));
        }
        w.push(Widget::Entity(EntityWidget { id: 0, entity: "column".into(), height: 24, follow_cursor: false }));
        if self.shown.is_empty() {
            let empty = if self.tab == 4 { t[11] } else { t[12] };
            w.push(Widget::Text(TextWidget { id: 0, text: empty.into(), color: 0xAAAAAA }));
        } else {
            if w.len() + self.shown.len() < MAX_WIDGETS {
                w.push(Widget::Text(TextWidget { id: 0, text: self.current(), color: 0xB98CF2 }));
            }
            let room = MAX_WIDGETS - w.len();
            for (k, &i) in self.shown.iter().enumerate().take(room) {
                w.push(Widget::Entity(EntityWidget {
                    id: CAPE + k as u32,
                    entity: format!("cape:capes/{}.png", CAPES[i].0),
                    height: 56,
                    follow_cursor: false,
                }));
            }
        }
        w
    }

    fn current(&self) -> String {
        let t = strings();
        format!("{}: {}", t[10], self.cape.map_or(t[7], |i| CAPES[i].1))
    }

    fn refresh(&mut self) {
        let widgets = self.widgets();
        ui::set_settings_v3(&widgets);
        if self.screen {
            let title = format!("{} - {}", strings()[0], self.current());
            ui::open_screen_v3(&ScreenV3 { title, widgets });
        }
    }
}

impl Mod for Acv {
    fn init() -> Result<(Self, Registration), String> {
        input::register_key(KEY, strings()[1], "key.keyboard.r")?;
        let text = storage::get(FILE).and_then(|v| String::from_utf8(v).ok()).unwrap_or_default();
        let get = |k: &str| text.lines().find_map(|l| l.strip_prefix(k).and_then(|r| r.strip_prefix('=')).map(str::trim));
        let find = |id: &str| CAPES.iter().position(|c| c.0 == id);
        let cape = get("cape").and_then(find);
        let tab = get("tab").and_then(|v| v.parse().ok()).filter(|t| *t <= 4).unwrap_or(0);
        let favorites = get("favorites").map(|v| v.split(',').filter_map(find).collect()).unwrap_or_default();
        let mut acv = Acv { cape, tab, favorites, shown: Vec::new(), screen: false };
        acv.wear();
        acv.refresh();
        Ok((acv, Registration::new().on_frame()))
    }

    fn frame(&mut self, _seconds: f32) {
        for event in input::poll() {
            if let Event::KeyDown(k) = event
                && k == KEY
            {
                self.screen = true;
                self.refresh();
            }
        }
    }

    fn ui_select(&mut self, id: u32, row: u32) {
        match id {
            c if c >= CAPE => {
                let Some(&i) = self.shown.get((c - CAPE) as usize) else { return };
                let _ = row;
                self.cape = Some(i);
                self.wear();
            }
            NONE => {
                self.cape = None;
                self.wear();
            }
            FAVORITE => {
                let Some(c) = self.cape else { return };
                if let Some(p) = self.favorites.iter().position(|&f| f == c) {
                    self.favorites.remove(p);
                } else {
                    self.favorites.push(c);
                }
            }
            t if (TABS..TABS + 5).contains(&t) => self.tab = t - TABS,
            _ => return,
        }
        self.save();
        self.refresh();
    }

    fn ui_closed(&mut self) {
        self.screen = false;
    }

    fn ui_settings_opened(&mut self) {
        self.screen = true;
        self.refresh();
    }
}

horizon_mod_sdk::export_mod!(Acv);
