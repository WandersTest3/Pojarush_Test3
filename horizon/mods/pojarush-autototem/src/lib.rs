use horizon_mod_sdk::{
    Mod, Registration, chat,
    input::{self, Event, ScreenKind},
    render::{self, ItemShape, Point, Shape, TextShape},
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
const COUNTER: u32 = 5;
const OPEN: u32 = 6;
const CLOSE: u32 = 7;
const RESET: u32 = 9;
const INFO: u32 = 10;
const STEP_MS: u32 = 25;
const DELAY_MAX: u32 = 1000;
const CLOSE_MAX: u32 = 1000;
const OPEN_TIMEOUT_MS: f32 = 1000.0;
const COOLDOWN_MS: f32 = 300.0;

const TEXT: [[&str; 15]; 3] = [
    ["Auto Totem", "Next totem delay", "ms", "Use hotbar totems", "Replace off-hand item", "Totem count in chat", "Reset to defaults", "On", "Off", "Totems left", "No totems left!", "Toggle Auto Totem", "Totem counter above XP", "Open inventory to swap", "Close inventory after"],
    ["Auto Totem", "Atraso do próximo totem", "ms", "Usar totens da hotbar", "Trocar item da mão secundária", "Contagem de totens no chat", "Restaurar padrões", "Ligado", "Desligado", "Totens restantes", "Acabaram os totens!", "Ligar/desligar Auto Totem", "Contador de totens acima do XP", "Abrir o inventário para trocar", "Fechar o inventário após"],
    ["Auto Totem", "Atraso do próximo totem", "ms", "Usar totens da barra rápida", "Trocar item da mão secundária", "Contagem de totens no chat", "Repor predefinições", "Ligado", "Desligado", "Totens restantes", "Acabaram os totens!", "Ligar/desligar Auto Totem", "Contador de totens acima do XP", "Abrir o inventário para trocar", "Fechar o inventário após"],
];

enum Step {
    Idle,
    Waiting(f32),
    Opening(f32),
    Closing(f32),
    Cooldown(f32),
}

const SETTINGS: [u32; 8] = [ENABLED, DELAY, HOTBAR, REPLACE, NOTIFY, COUNTER, OPEN, CLOSE];

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
    counter: bool,
    open: bool,
    close: u32,
    step: Step,
    had_totem: bool,
    shown: Option<(u32, (f32, f32))>,
}

impl AutoTotem {
    fn defaults() -> Self {
        Self {
            enabled: true,
            delay: 0,
            hotbar: true,
            replace: false,
            notify: true,
            counter: true,
            open: false,
            close: 100,
            step: Step::Idle,
            had_totem: false,
            shown: None,
        }
    }

    fn load() -> Self {
        let mut s = Self::defaults();
        if let Some(text) = storage::get(FILE).and_then(|v| String::from_utf8(v).ok()) {
            let mut ms = None;
            for line in text.lines() {
                let Some((k, v)) = line.split_once('=') else { continue };
                let n: u32 = v.trim().parse().unwrap_or(0);
                match k.trim() {
                    "enabled" => s.enabled = n == 1,
                    "delay" if ms.is_none() => s.delay = (n * 50).min(DELAY_MAX),
                    "delay_ms" => {
                        ms = Some(());
                        s.delay = n.min(DELAY_MAX) / STEP_MS * STEP_MS;
                    }
                    "open_inventory" => s.open = n == 1,
                    "close_ms" => s.close = n.min(CLOSE_MAX) / STEP_MS * STEP_MS,
                    "hotbar" => s.hotbar = n == 1,
                    "replace" => s.replace = n == 1,
                    "notify" => s.notify = n == 1,
                    "counter" => s.counter = n == 1,
                    _ => {}
                }
            }
        }
        s
    }

    fn save(&self) {
        let text = format!(
            "enabled={}\ndelay_ms={}\nopen_inventory={}\nclose_ms={}\nhotbar={}\nreplace={}\nnotify={}\ncounter={}\n",
            u32::from(self.enabled),
            self.delay,
            u32::from(self.open),
            self.close,
            u32::from(self.hotbar),
            u32::from(self.replace),
            u32::from(self.notify),
            u32::from(self.counter),
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
                steps: DELAY_MAX / STEP_MS,
                position: self.delay / STEP_MS,
            }),
            Widget::Button(ButtonWidget { id: OPEN, label: format!("{}: {}", t[13], on_off(self.open)) }),
            Widget::Slider(SliderWidget {
                id: CLOSE,
                label: format!("{}: {} {}", t[14], self.close, t[2]),
                steps: CLOSE_MAX / STEP_MS,
                position: self.close / STEP_MS,
            }),
            Widget::Button(ButtonWidget { id: HOTBAR, label: format!("{}: {}", t[3], on_off(self.hotbar)) }),
            Widget::Button(ButtonWidget { id: REPLACE, label: format!("{}: {}", t[4], on_off(self.replace)) }),
            Widget::Button(ButtonWidget { id: NOTIFY, label: format!("{}: {}", t[5], on_off(self.notify)) }),
            Widget::Button(ButtonWidget { id: COUNTER, label: format!("{}: {}", t[12], on_off(self.counter)) }),
            Widget::Button(ButtonWidget { id: RESET, label: t[6].into() }),
            Widget::Text(TextWidget { id: INFO, text: format!("{}: {}", t[9], self.count()), color: 0xB98CF2 }),
        ];
        ui::set_settings_v2(&widgets);
        ui::set_resets(&SETTINGS);
    }

    fn count(&self) -> u32 {
        input::inventory().iter().filter(|s| s.id == TOTEM).map(|s| s.count).sum()
    }

    fn reset(&mut self, id: u32) {
        let d = Self::defaults();
        match id {
            ENABLED => self.enabled = d.enabled,
            DELAY => self.delay = d.delay,
            HOTBAR => self.hotbar = d.hotbar,
            REPLACE => self.replace = d.replace,
            NOTIFY => self.notify = d.notify,
            COUNTER => self.counter = d.counter,
            OPEN => self.open = d.open,
            CLOSE => self.close = d.close,
            _ => {}
        }
    }

    /// The totems held: the inventory's and the off hand's, with a totem icon above the XP level
    /// (where vanilla draws the level, between the hearts and the food).
    fn draw(&mut self) {
        let count = self.count();
        let size = render::gui_size();
        let shown = (self.enabled && self.counter).then_some((count, size));
        if shown == self.shown {
            return;
        }
        self.shown = shown;
        if shown.is_none() {
            render::set_hud(&[]);
            return;
        }
        let (w, h) = size;
        let x = (w / 2.0).floor();
        let y = h - 48.0;
        let color = if count == 0 { 0xFF5555 } else { 0xFFFF55 };
        render::set_hud(&[
            Shape::Item(ItemShape { item: TOTEM.into(), at: Point { x: x - 11.0, y: y - 1.0 }, scale: 0.625 }),
            Shape::Text(TextShape {
                text: count.to_string(),
                at: Point { x: x + 1.0, y },
                color,
                centered: false,
                shadow: true,
            }),
        ]);
    }

    /// The inventory slot a totem comes from, or none when the off hand needs nothing.
    fn source(&mut self) -> Option<u32> {
        let stacks = input::inventory();
        let offhand = stacks.iter().find(|s| s.slot == OFFHAND);
        if offhand.is_some_and(|s| s.id == TOTEM) {
            self.had_totem = true;
            return None;
        }
        if offhand.is_some() && !self.replace {
            return None;
        }
        let source = stacks
            .iter()
            .filter(|s| s.id == TOTEM && s.slot < 36 && (self.hotbar || s.slot >= 9))
            .min_by_key(|s| if s.slot >= 9 { s.slot } else { 100 + s.slot });
        if source.is_none() {
            if self.had_totem && self.notify {
                chat::show(TEXT[lang()][10]);
            }
            self.had_totem = false;
        }
        source.map(|s| s.slot)
    }

    fn swap(&mut self, slot: u32) -> bool {
        if input::swap_offhand(slot).is_err() {
            return false;
        }
        if self.had_totem && self.notify {
            let left: u32 = input::inventory()
                .iter()
                .filter(|s| s.id == TOTEM && s.slot < 36 && s.slot != slot)
                .map(|s| s.count)
                .sum();
            chat::show(&format!("{}: {left}", TEXT[lang()][9]));
        }
        self.had_totem = true;
        true
    }

    /// Once a frame: waits the delay, then swaps; with "open inventory" on it opens the
    /// inventory first (unless it is open already), swaps and closes it after the close delay.
    fn refill(&mut self, ms: f32) {
        let screen = input::current_screen();
        self.step = match std::mem::replace(&mut self.step, Step::Idle) {
            Step::Idle => match self.source() {
                Some(_) => Step::Waiting(0.0),
                None => Step::Idle,
            },
            Step::Waiting(t) => {
                let t = t + ms;
                if t < self.delay as f32 {
                    Step::Waiting(t)
                } else {
                    match (self.source(), screen) {
                        (None, _) => Step::Idle,
                        (Some(_), ScreenKind::Other) => Step::Waiting(t),
                        (Some(_), ScreenKind::Playing) if self.open => {
                            input::open_inventory();
                            Step::Opening(0.0)
                        }
                        (Some(slot), _) => {
                            self.swap(slot);
                            Step::Cooldown(0.0)
                        }
                    }
                }
            }
            Step::Opening(t) => match screen {
                ScreenKind::Inventory => match self.source() {
                    Some(slot) if self.swap(slot) => Step::Closing(0.0),
                    _ => Step::Closing(self.close as f32),
                },
                _ if t + ms >= OPEN_TIMEOUT_MS => Step::Idle,
                _ => Step::Opening(t + ms),
            },
            Step::Closing(t) => {
                let t = t + ms;
                if !matches!(screen, ScreenKind::Inventory) {
                    Step::Cooldown(0.0)
                } else if t >= self.close as f32 {
                    input::close_inventory();
                    Step::Cooldown(0.0)
                } else {
                    Step::Closing(t)
                }
            }
            Step::Cooldown(t) if t + ms < COOLDOWN_MS => Step::Cooldown(t + ms),
            Step::Cooldown(_) => Step::Idle,
        };
    }
}

impl Mod for AutoTotem {
    fn init() -> Result<(Self, Registration), String> {
        let m = AutoTotem::load();
        let _ = input::register_key(KEY, TEXT[lang()][11], "key.keyboard.unknown");
        m.page();
        Ok((m, Registration::new().on_tick().on_frame()))
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
        self.draw();
    }

    fn frame(&mut self, seconds: f32) {
        if self.enabled || matches!(self.step, Step::Closing(_)) {
            self.refill(seconds * 1000.0);
        } else {
            self.step = Step::Idle;
        }
    }

    fn ui_select(&mut self, id: u32, row: u32) {
        if row == u32::MAX {
            self.reset(id);
        } else {
            match id {
                ENABLED => self.enabled = !self.enabled,
                DELAY => self.delay = (row * STEP_MS).min(DELAY_MAX),
                OPEN => self.open = !self.open,
                CLOSE => self.close = (row * STEP_MS).min(CLOSE_MAX),
                HOTBAR => self.hotbar = !self.hotbar,
                REPLACE => self.replace = !self.replace,
                NOTIFY => self.notify = !self.notify,
                COUNTER => self.counter = !self.counter,
                RESET => {
                    for id in SETTINGS {
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
