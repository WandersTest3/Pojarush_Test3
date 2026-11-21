use horizon_mod_sdk::{
    Mod, Registration,
    input::{self, Event},
    render::{self, Zoom as ZoomView},
    storage,
    ui::{self, ButtonWidget, SliderWidget, WidgetV2 as Widget},
};

const KEY: &str = "zoom";
const MIN: f32 = 0.10;
const MAX: f32 = 200.0;
const LEVEL_MIN: u32 = 2;
const LEVEL_MAX: u32 = 30;
const SPEED_MIN: u32 = 1;
const SPEED_MAX: u32 = 10;
const STEPS: [f32; 6] = [1.10, 1.25, 1.50, 2.00, 3.00, 4.00];
const LEVEL_DOWN: u32 = 1;
const LEVEL_UP: u32 = 2;
const SPEED_DOWN: u32 = 3;
const SPEED_UP: u32 = 4;
const S_LEVEL: u32 = 20;
const S_SPEED: u32 = 21;
const WHEEL_STEP: u32 = 5;
const MODE: u32 = 6;
const STEADY: u32 = 8;
const RESET: u32 = 9;

struct Zoom {
    level: u32,
    speed: u32,
    step: u32,
    toggle: bool,
    steady: bool,
    active: bool,
    target: f32,
    current: f32,
}

const LANGS: [(&str, [&str; 10]); 19] = [
    ("en", ["Zoom distance", "Zoom speed", "Wheel step", "Zoom key", "Toggle", "Hold", "Slower camera while zoomed", "Reset to defaults", "On", "Off"]),
    ("pt_br", ["Distância do zoom", "Velocidade do zoom", "Passo da roda", "Tecla de zoom", "Alternar", "Segurar", "Câmera mais lenta com zoom", "Restaurar padrões", "Ligado", "Desligado"]),
    ("pt_pt", ["Distância do zoom", "Velocidade do zoom", "Passo da roda", "Tecla de zoom", "Alternar", "Manter premida", "Câmara mais lenta com zoom", "Repor predefinições", "Ligado", "Desligado"]),
    ("es", ["Distancia del zoom", "Velocidad del zoom", "Paso de la rueda", "Tecla de zoom", "Alternar", "Mantener", "Cámara más lenta con zoom", "Restablecer valores", "Activado", "Desactivado"]),
    ("fr", ["Distance du zoom", "Vitesse du zoom", "Pas de la molette", "Touche de zoom", "Basculer", "Maintenir", "Caméra plus lente en zoom", "Rétablir les valeurs", "Activé", "Désactivé"]),
    ("de", ["Zoomweite", "Zoomgeschwindigkeit", "Mausrad-Schritt", "Zoomtaste", "Umschalten", "Halten", "Langsamere Kamera beim Zoomen", "Auf Standard zurücksetzen", "An", "Aus"]),
    ("it", ["Distanza dello zoom", "Velocità dello zoom", "Passo della rotella", "Tasto zoom", "Alterna", "Tieni premuto", "Telecamera più lenta con lo zoom", "Ripristina predefiniti", "Attivo", "Disattivo"]),
    ("ru", ["Дальность зума", "Скорость зума", "Шаг колёсика", "Клавиша зума", "Переключение", "Удержание", "Медленная камера при зуме", "Сбросить настройки", "Вкл.", "Выкл."]),
    ("uk", ["Дальність зуму", "Швидкість зуму", "Крок коліщатка", "Клавіша зуму", "Перемикання", "Утримання", "Повільніша камера під час зуму", "Скинути налаштування", "Увімк.", "Вимк."]),
    ("pl", ["Odległość zoomu", "Szybkość zoomu", "Krok kółka", "Klawisz zoomu", "Przełączanie", "Przytrzymanie", "Wolniejsza kamera przy zoomie", "Przywróć domyślne", "Wł.", "Wył."]),
    ("tr", ["Yakınlaştırma mesafesi", "Yakınlaştırma hızı", "Tekerlek adımı", "Yakınlaştırma tuşu", "Aç/Kapat", "Basılı tut", "Yakınlaştırmada yavaş kamera", "Varsayılanlara dön", "Açık", "Kapalı"]),
    ("ar", ["مسافة التكبير", "سرعة التكبير", "خطوة العجلة", "مفتاح التكبير", "تبديل", "ضغط مستمر", "كاميرا أبطأ أثناء التكبير", "استعادة الإعدادات الافتراضية", "مفعّل", "معطّل"]),
    ("zh_cn", ["缩放距离", "缩放速度", "滚轮步长", "缩放键", "切换", "按住", "缩放时降低镜头速度", "恢复默认设置", "开", "关"]),
    ("zh_tw", ["縮放距離", "縮放速度", "滾輪步長", "縮放鍵", "切換", "按住", "縮放時降低鏡頭速度", "恢復預設值", "開", "關"]),
    ("ja", ["ズーム距離", "ズーム速度", "ホイールの刻み", "ズームキー", "切り替え", "長押し", "ズーム中はカメラを遅く", "初期設定に戻す", "オン", "オフ"]),
    ("ko", ["확대 거리", "확대 속도", "휠 단계", "확대 키", "전환", "누르고 있기", "확대 중 카메라 감속", "기본값 복원", "켜짐", "꺼짐"]),
    ("th", ["ระยะซูม", "ความเร็วซูม", "ระดับล้อเมาส์", "ปุ่มซูม", "สลับ", "กดค้าง", "กล้องช้าลงขณะซูม", "คืนค่าเริ่มต้น", "เปิด", "ปิด"]),
    ("vi", ["Khoảng cách phóng to", "Tốc độ phóng to", "Bước con lăn", "Phím phóng to", "Bật/tắt", "Giữ", "Camera chậm hơn khi phóng to", "Khôi phục mặc định", "Bật", "Tắt"]),
    ("id", ["Jarak zoom", "Kecepatan zoom", "Langkah roda", "Tombol zoom", "Alihkan", "Tahan", "Kamera lebih lambat saat zoom", "Kembalikan bawaan", "Nyala", "Mati"]),
];

fn strings() -> &'static [&'static str; 10] {
    let code = ui::language().to_lowercase();
    let find = |c: &str| LANGS.iter().find(|(l, _)| *l == c).map(|(_, s)| s);
    find(&code)
        .or_else(|| code.split('_').next().and_then(find))
        .or_else(|| match code.split('_').next() {
            Some("pt") => find("pt_br"),
            Some("zh") => find("zh_cn"),
            _ => None,
        })
        .unwrap_or(&LANGS[0].1)
}

fn on_off(t: &[&'static str; 10], on: bool) -> &'static str {
    if on { t[8] } else { t[9] }
}

const FILE: &str = "pojarush_zoom.txt";
const OLD_KEYS: [&str; 6] = ["level", "speed", "step", "toggle", "keep", "steady"];

fn load_config() -> Vec<(String, String)> {
    if let Some(text) = storage::get(FILE).and_then(|v| String::from_utf8(v).ok()) {
        return text
            .lines()
            .filter_map(|l| {
                let l = l.trim();
                if l.is_empty() || l.starts_with('#') {
                    return None;
                }
                let (k, v) = l.split_once('=')?;
                Some((k.trim().to_owned(), v.trim().to_owned()))
            })
            .collect();
    }
    OLD_KEYS
        .iter()
        .filter_map(|k| {
            storage::get(k)
                .and_then(|v| String::from_utf8(v).ok())
                .map(|v| ((*k).to_owned(), v.trim().to_owned()))
        })
        .collect()
}

fn stored(cfg: &[(String, String)], key: &str, default: u32, min: u32, max: u32) -> u32 {
    cfg.iter()
        .find(|(k, _)| k == key)
        .and_then(|(_, v)| v.parse::<u32>().ok())
        .map_or(default, |v| v.clamp(min, max))
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

    fn slider(&self, id: u32) -> Widget {
        let (label, steps, position) = self.slider_label(id);
        Widget::Slider(SliderWidget { id, label, steps, position })
    }

    fn slider_label(&self, id: u32) -> (String, u32, u32) {
        match id {
            S_LEVEL => (
                format!("{}: {}x", strings()[0], self.level),
                LEVEL_MAX - LEVEL_MIN,
                self.level - LEVEL_MIN,
            ),
            _ => (
                format!("{}: {}", strings()[1], self.speed),
                SPEED_MAX - SPEED_MIN,
                self.speed - SPEED_MIN,
            ),
        }
    }

    fn page(&self) {
        let t = strings();
        ui::set_settings_v2(&[
            self.slider(S_LEVEL),
            self.slider(S_SPEED),
            Self::button(
                WHEEL_STEP,
                format!("{}: {:.2}x", t[2], STEPS[self.step as usize]),
            ),
            Self::button(
                MODE,
                format!("{}: {}", t[3], if self.toggle { t[4] } else { t[5] }),
            ),
            Self::button(
                STEADY,
                format!("{}: {}", t[6], on_off(t, self.steady)),
            ),
            Self::button(RESET, t[7].into()),
        ]);
        ui::set_resets(&[S_LEVEL, S_SPEED, WHEEL_STEP, MODE, STEADY]);
    }

    fn save(&self) {
        let text = format!(
            "level={}\nspeed={}\nstep={}\ntoggle={}\nsteady={}\n",
            self.level,
            self.speed,
            self.step,
            u32::from(self.toggle),
            u32::from(self.steady),
        );
        if storage::set(FILE, text.as_bytes()).is_ok() {
            for k in OLD_KEYS {
                storage::delete(k);
            }
        }
    }

    fn rate(&self) -> f32 {
        2.4 * self.speed as f32
    }

    fn start(&mut self) {
        self.active = true;
        self.target = self.level as f32;
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
    }
}


impl Mod for Zoom {
    fn init() -> Result<(Self, Registration), String> {
        input::register_key(KEY, "Zoom", "key.keyboard.c")?;
        let cfg = load_config();
        let zoom = Zoom {
            level: stored(&cfg, "level", 8, LEVEL_MIN, LEVEL_MAX),
            speed: stored(&cfg, "speed", 6, SPEED_MIN, SPEED_MAX),
            step: stored(&cfg, "step", 2, 0, STEPS.len() as u32 - 1),
            toggle: stored(&cfg, "toggle", 0, 0, 1) == 1,
            steady: stored(&cfg, "steady", 1, 0, 1) == 1,
            active: false,
            target: 1.0,
            current: 1.0,
        };
        zoom.save();
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

    fn ui_select(&mut self, id: u32, row: u32) {
        // Shift + click: that setting back at its default.
        if row == u32::MAX {
            match id {
                S_LEVEL => self.level = 8,
                S_SPEED => self.speed = 6,
                WHEEL_STEP => self.step = 2,
                MODE => {
                    self.toggle = false;
                    if self.active {
                        self.stop();
                    }
                }
                STEADY => self.steady = true,
                _ => return,
            }
            self.save();
            self.page();
            return;
        }
        if id == S_LEVEL || id == S_SPEED {
            if id == S_LEVEL {
                self.level = (LEVEL_MIN + row).min(LEVEL_MAX);
            } else {
                self.speed = (SPEED_MIN + row).min(SPEED_MAX);
            }
            let (label, _, position) = self.slider_label(id);
            ui::set_slider(id, position, &label);
            self.save();
            return;
        }
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
            STEADY => self.steady = !self.steady,
            RESET => {
                self.level = 8;
                self.speed = 6;
                self.step = 2;
                self.toggle = false;
                self.steady = true;
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
