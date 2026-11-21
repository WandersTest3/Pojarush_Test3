use horizon_mod_sdk::{
    Mod, Registration, chat,
    input::{self, Event},
    storage,
    ui::{self, ButtonWidget, SliderWidget, TextWidget, WidgetV2 as Widget},
};

const FILE: &str = "autototem.txt";
const TOTEM: &str = "minecraft:totem_of_undying";
const OFFHAND: u32 = 40;
const KEY: &str = "toggle";

const ENABLED: u32 = 0;
const DELAY: u32 = 1;
const HOTBAR: u32 = 2;
const REPLACE: u32 = 3;
const NOTIFY: u32 = 4;
const RESET: u32 = 9;
const INFO: u32 = 10;
const DELAY_MAX: u32 = 20;

const TEXT: [[&str; 12]; 3] = [
    ["Auto Totem", "Delay", "ticks", "Use hotbar totems", "Replace off-hand item", "Totem count in chat", "Reset to defaults", "On", "Off", "Totems left", "No totems left!", "Toggle Auto Totem"],
    ["Auto Totem", "Atraso", "ticks", "Usar totens da hotbar", "Trocar item da mão secundária", "Contagem de totens no chat", "Restaurar padrões", "Ligado", "Desligado", "Totens restantes", "Acabaram os totens!", "Ligar/desligar Auto Totem"],
    ["Auto Totem", "Atraso", "ticks", "Usar totens da barra rápida", "Trocar item da mão secundária", "Contagem de totens no chat", "Repor predefinições", "Ligado", "Desligado", "Totens restantes", "Acabaram os totens!", "Ligar/desligar Auto Totem"],
];

fn lang() -> usize {
    match ui::language().as_str() {
        "pt_br" => 1,
        "pt_pt" => 2,
        _ => 0,
    }
}

struct AutoTotem {
    enabled: bool,
    delay: u32,
    hotbar: bool,
    replace: bool,
    notify: bool,
    wait: u32,
    had_totem: bool,
}

impl AutoTotem {
    fn load() -> Self {
        let mut s = Self { enabled: true, delay: 0, hotbar: true, replace: false, notify: true, wait: 0, had_totem: false };
        if let Some(text) = storage::get(FILE).and_then(|v| String::from_utf8(v).ok()) {
            for line in text.lines() {
                let Some((k, v)) = line.split_once('=') else { continue };
                let n: u32 = v.trim().parse().unwrap_or(0);
                match k.trim() {
                    "enabled" => s.enabled = n == 1,
                    "delay" => s.delay = n.min(DELAY_MAX),
                    "hotbar" => s.hotbar = n == 1,
                    "replace" => s.replace = n == 1,
                    "notify" => s.notify = n == 1,
                    _ => {}
                }
            }
        }
        s
    }

    fn save(&self) {
        let text = format!(
            "enabled={}\ndelay={}\nhotbar={}\nreplace={}\nnotify={}\n",
            u32::from(self.enabled),
            self.delay,
            u32::from(self.hotbar),
            u32::from(self.replace),
            u32::from(self.notify),
        );
        let _ = storage::set(FILE, text.as_bytes());
    }

    fn page(&self) {
        let t = TEXT[lang()];
        let on_off = |b: bool| if b { t[7] } else { t[8] };
        let widgets = vec![
            Widget::Button(ButtonWidget { id: ENABLED, label: format!("{}: {}", t[0], on_off(self.enabled)) }),
            Widget::Slider(SliderWidget {
                id: DELAY,
                label: format!("{}: {} {}", t[1], self.delay, t[2]),
                steps: DELAY_MAX,
                position: self.delay,
            }),
            Widget::Button(ButtonWidget { id: HOTBAR, label: format!("{}: {}", t[3], on_off(self.hotbar)) }),
            Widget::Button(ButtonWidget { id: REPLACE, label: format!("{}: {}", t[4], on_off(self.replace)) }),
            Widget::Button(ButtonWidget { id: NOTIFY, label: format!("{}: {}", t[5], on_off(self.notify)) }),
            Widget::Button(ButtonWidget { id: RESET, label: t[6].into() }),
            Widget::Text(TextWidget { id: INFO, text: format!("{}: {}", t[9], self.count()), color: 0xB98CF2 }),
        ];
        ui::set_settings_v2(&widgets);
        ui::set_resets(&[ENABLED, DELAY, HOTBAR, REPLACE, NOTIFY]);
    }

    fn count(&self) -> u32 {
        input::inventory().iter().filter(|s| s.id == TOTEM).map(|s| s.count).sum()
    }

    fn reset(&mut self, id: u32) {
        let d = Self { enabled: true, delay: 0, hotbar: true, replace: false, notify: true, wait: 0, had_totem: false };
        match id {
            ENABLED => self.enabled = d.enabled,
            DELAY => self.delay = d.delay,
            HOTBAR => self.hotbar = d.hotbar,
            REPLACE => self.replace = d.replace,
            NOTIFY => self.notify = d.notify,
            _ => {}
        }
    }

    fn refill(&mut self) {
        let stacks = input::inventory();
        let offhand = stacks.iter().find(|s| s.slot == OFFHAND);
        let holding = offhand.is_some_and(|s| s.id == TOTEM);
        if holding {
            self.had_totem = true;
            self.wait = 0;
            return;
        }
        if offhand.is_some() && !self.replace {
            self.wait = 0;
            return;
        }
        let source = stacks
            .iter()
            .filter(|s| s.id == TOTEM && s.slot < 36 && (self.hotbar || s.slot >= 9))
            .min_by_key(|s| if s.slot >= 9 { s.slot } else { 100 + s.slot });
        let Some(source) = source else {
            if self.had_totem && self.notify {
                chat::show(TEXT[lang()][10]);
            }
            self.had_totem = false;
            return;
        };
        if self.wait < self.delay {
            self.wait += 1;
            return;
        }
        self.wait = 0;
        if input::swap_offhand(source.slot).is_ok() {
            if self.had_totem && self.notify {
                let left: u32 = stacks.iter().filter(|s| s.id == TOTEM && s.slot < 36).map(|s| s.count).sum();
                chat::show(&format!("{}: {left}", TEXT[lang()][9]));
            }
            self.had_totem = true;
        }
    }
}

impl Mod for AutoTotem {
    fn init() -> Result<(Self, Registration), String> {
        let m = AutoTotem::load();
        let _ = input::register_key(KEY, TEXT[lang()][11], "key.keyboard.unknown");
        m.page();
        Ok((m, Registration::new().on_tick()))
    }

    fn tick(&mut self, _tick: u64) {
        for event in input::poll() {
            if let Event::KeyDown(k) = &event
                && k == KEY
            {
                self.enabled = !self.enabled;
                let t = TEXT[lang()];
                chat::show(&format!("{}: {}", t[0], if self.enabled { t[7] } else { t[8] }));
                self.save();
                self.page();
            }
        }
        if self.enabled {
            self.refill();
        }
    }

    fn ui_select(&mut self, id: u32, row: u32) {
        if row == u32::MAX {
            self.reset(id);
        } else {
            match id {
                ENABLED => self.enabled = !self.enabled,
                DELAY => self.delay = row.min(DELAY_MAX),
                HOTBAR => self.hotbar = !self.hotbar,
                REPLACE => self.replace = !self.replace,
                NOTIFY => self.notify = !self.notify,
                RESET => {
                    for id in [ENABLED, DELAY, HOTBAR, REPLACE, NOTIFY] {
                        self.reset(id);
                    }
                }
                _ => return,
            }
        }
        self.save();
        self.page();
    }

    fn ui_settings_opened(&mut self) {
        self.page();
    }
}

horizon_mod_sdk::export_mod!(AutoTotem);
