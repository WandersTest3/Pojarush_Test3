use horizon_mod_sdk::{
    Mod, Registration,
    render::{self, FillShape, Point, Rect, Shape, TextShape},
    storage,
    ui::{self, ButtonWidget, SliderWidget, WidgetV2 as Widget},
};

const COLORS: [(&str, u32); 5] = [
    ("White", 0xFFFFFF),
    ("Purple", 0xB98CF2),
    ("Green", 0x55FF55),
    ("Yellow", 0xFFFF55),
    ("By FPS", 0),
];
const CORNERS: [(&str, u32, u32); 5] = [
    ("Top left", 0, 0),
    ("Top right", 1000, 0),
    ("Bottom left", 0, 1000),
    ("Bottom right", 1000, 1000),
    ("Top center", 500, 0),
];

const SCALE_MIN: u32 = 25;
const SCALE_MAX: u32 = 200;
const SCALE_STEP: u32 = 5;
const PAD_MAX: u32 = 32;
const SHOW: u32 = 1;
const CORNER: u32 = 8;
const LOWS: u32 = 9;
const BACKGROUND: u32 = 10;
const COLOR: u32 = 11;
const RESET: u32 = 12;
const S_SIZE: u32 = 20;
const S_X: u32 = 21;
const S_Y: u32 = 22;
const S_PAD: u32 = 25;
const S_OPACITY: u32 = 26;

struct Fps {
    show: bool,
    scale: u32,
    x: u32,
    y: u32,
    pad: u32,
    opacity: u32,
    lows: bool,
    background: bool,
    color: u32,
    last: (u32, u32, u32, u32, u32),
    drawn: bool,
    area: (f32, f32, f32, f32, f32, f32),
}

fn on_off(s: &[&'static str; 25], on: bool) -> &'static str {
    if on { s[13] } else { s[14] }
}

const FILE: &str = "pojarush_fps.txt";
const OLD_KEYS: [&str; 15] = [
    "show", "scale", "size", "x", "y", "xt", "yt", "xoff", "yoff", "pad", "opacity", "lows",
    "background", "color", "corner",
];

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

fn raw<'a>(cfg: &'a [(String, String)], key: &str) -> Option<&'a str> {
    cfg.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
}

fn stored(cfg: &[(String, String)], key: &str, default: u32, min: u32, max: u32) -> u32 {
    raw(cfg, key)
        .and_then(|v| v.parse::<u32>().ok())
        .map_or(default, |v| v.clamp(min, max))
}

const LANGS: [(&str, [&str; 25]); 19] = [
    ("en", ["FPS counter", "Minimum and 1% low", "Background", "Color", "Move to the next corner", "Reset to defaults", "Size", "Horizontal position", "Vertical position", "Move with the mouse...", "Drag the FPS counter, then press Done", "Background padding", "Background opacity", "On", "Off", "White", "Purple", "Green", "Yellow", "By FPS", "Top left", "Top right", "Bottom left", "Bottom right", "Done"]),
    ("pt_br", ["Contador de FPS", "Mínimo e 1% low", "Fundo", "Cor", "Mover para o próximo canto", "Restaurar padrões", "Tamanho", "Posição horizontal", "Posição vertical", "Mover com o mouse...", "Arraste o contador de FPS e toque em Concluído", "Margem do fundo", "Opacidade do fundo", "Ligado", "Desligado", "Branco", "Roxo", "Verde", "Amarelo", "Pelo FPS", "Canto superior esquerdo", "Canto superior direito", "Canto inferior esquerdo", "Canto inferior direito", "Concluído"]),
    ("pt_pt", ["Contador de FPS", "Mínimo e 1% low", "Fundo", "Cor", "Mover para o canto seguinte", "Repor predefinições", "Tamanho", "Posição horizontal", "Posição vertical", "Mover com o rato...", "Arraste o contador de FPS e toque em Concluído", "Margem do fundo", "Opacidade do fundo", "Ligado", "Desligado", "Branco", "Roxo", "Verde", "Amarelo", "Pelo FPS", "Canto superior esquerdo", "Canto superior direito", "Canto inferior esquerdo", "Canto inferior direito", "Concluído"]),
    ("es", ["Contador de FPS", "Mínimo y 1% low", "Fondo", "Color", "Mover a la esquina siguiente", "Restablecer valores", "Tamaño", "Posición horizontal", "Posición vertical", "Mover con el ratón...", "Arrastra el contador de FPS y pulsa Listo", "Margen del fondo", "Opacidad del fondo", "Activado", "Desactivado", "Blanco", "Morado", "Verde", "Amarillo", "Según FPS", "Arriba a la izquierda", "Arriba a la derecha", "Abajo a la izquierda", "Abajo a la derecha", "Listo"]),
    ("fr", ["Compteur de FPS", "Minimum et 1% low", "Fond", "Couleur", "Aller au coin suivant", "Rétablir les valeurs", "Taille", "Position horizontale", "Position verticale", "Déplacer à la souris...", "Faites glisser le compteur de FPS, puis appuyez sur Terminé", "Marge du fond", "Opacité du fond", "Activé", "Désactivé", "Blanc", "Violet", "Vert", "Jaune", "Selon les FPS", "En haut à gauche", "En haut à droite", "En bas à gauche", "En bas à droite", "Terminé"]),
    ("de", ["FPS-Anzeige", "Minimum und 1% low", "Hintergrund", "Farbe", "Zur nächsten Ecke", "Auf Standard zurücksetzen", "Größe", "Horizontale Position", "Vertikale Position", "Mit der Maus verschieben...", "Ziehe den FPS-Zähler und drücke dann Fertig", "Hintergrundrand", "Hintergrunddeckkraft", "An", "Aus", "Weiß", "Lila", "Grün", "Gelb", "Nach FPS", "Oben links", "Oben rechts", "Unten links", "Unten rechts", "Fertig"]),
    ("it", ["Contatore FPS", "Minimo e 1% low", "Sfondo", "Colore", "Vai all'angolo successivo", "Ripristina predefiniti", "Dimensione", "Posizione orizzontale", "Posizione verticale", "Sposta con il mouse...", "Trascina il contatore FPS, poi premi Fatto", "Margine dello sfondo", "Opacità dello sfondo", "Attivo", "Disattivo", "Bianco", "Viola", "Verde", "Giallo", "In base agli FPS", "In alto a sinistra", "In alto a destra", "In basso a sinistra", "In basso a destra", "Fatto"]),
    ("ru", ["Счётчик FPS", "Минимум и 1% low", "Фон", "Цвет", "В следующий угол", "Сбросить настройки", "Размер", "Горизонтальная позиция", "Вертикальная позиция", "Переместить мышью...", "Перетащите счётчик FPS и нажмите «Готово»", "Отступ фона", "Непрозрачность фона", "Вкл.", "Выкл.", "Белый", "Фиолетовый", "Зелёный", "Жёлтый", "По FPS", "Сверху слева", "Сверху справа", "Снизу слева", "Снизу справа", "Готово"]),
    ("uk", ["Лічильник FPS", "Мінімум і 1% low", "Тло", "Колір", "До наступного кута", "Скинути налаштування", "Розмір", "Горизонтальна позиція", "Вертикальна позиція", "Перемістити мишею...", "Перетягніть лічильник FPS і натисніть «Готово»", "Відступ тла", "Непрозорість тла", "Увімк.", "Вимк.", "Білий", "Фіолетовий", "Зелений", "Жовтий", "За FPS", "Вгорі ліворуч", "Вгорі праворуч", "Внизу ліворуч", "Внизу праворуч", "Готово"]),
    ("pl", ["Licznik FPS", "Minimum i 1% low", "Tło", "Kolor", "Do następnego rogu", "Przywróć domyślne", "Rozmiar", "Pozycja pozioma", "Pozycja pionowa", "Przesuń myszą...", "Przeciągnij licznik FPS, potem naciśnij Gotowe", "Margines tła", "Krycie tła", "Wł.", "Wył.", "Biały", "Fioletowy", "Zielony", "Żółty", "Według FPS", "Lewy górny", "Prawy górny", "Lewy dolny", "Prawy dolny", "Gotowe"]),
    ("tr", ["FPS sayacı", "Minimum ve 1% low", "Arka plan", "Renk", "Sonraki köşeye taşı", "Varsayılanlara dön", "Boyut", "Yatay konum", "Dikey konum", "Fareyle taşı...", "FPS sayacını sürükleyin, sonra Bitti'ye basın", "Arka plan boşluğu", "Arka plan opaklığı", "Açık", "Kapalı", "Beyaz", "Mor", "Yeşil", "Sarı", "FPS'ye göre", "Sol üst", "Sağ üst", "Sol alt", "Sağ alt", "Bitti"]),
    ("ar", ["عداد FPS", "الحد الأدنى و1% low", "الخلفية", "اللون", "الانتقال إلى الزاوية التالية", "استعادة الإعدادات الافتراضية", "الحجم", "الموضع الأفقي", "الموضع العمودي", "التحريك بالفأرة...", "اسحب عداد FPS ثم اضغط تم", "هامش الخلفية", "شفافية الخلفية", "مفعّل", "معطّل", "أبيض", "بنفسجي", "أخضر", "أصفر", "حسب FPS", "أعلى اليسار", "أعلى اليمين", "أسفل اليسار", "أسفل اليمين", "تم"]),
    ("zh_cn", ["FPS 计数器", "最低值与 1% low", "背景", "颜色", "移到下一个角落", "恢复默认设置", "大小", "水平位置", "垂直位置", "用鼠标移动...", "拖动 FPS 计数器，然后点击完成", "背景边距", "背景不透明度", "开", "关", "白色", "紫色", "绿色", "黄色", "按 FPS", "左上", "右上", "左下", "右下", "完成"]),
    ("zh_tw", ["FPS 計數器", "最低值與 1% low", "背景", "顏色", "移到下一個角落", "恢復預設值", "大小", "水平位置", "垂直位置", "用滑鼠移動...", "拖曳 FPS 計數器，然後點擊完成", "背景邊距", "背景不透明度", "開", "關", "白色", "紫色", "綠色", "黃色", "依 FPS", "左上", "右上", "左下", "右下", "完成"]),
    ("ja", ["FPS カウンター", "最小値と 1% low", "背景", "色", "次の角へ移動", "初期設定に戻す", "サイズ", "水平位置", "垂直位置", "マウスで移動...", "FPS カウンターをドラッグして「完了」を押してください", "背景の余白", "背景の不透明度", "オン", "オフ", "白", "紫", "緑", "黄", "FPS に応じて", "左上", "右上", "左下", "右下", "完了"]),
    ("ko", ["FPS 카운터", "최소값과 1% low", "배경", "색상", "다음 모서리로 이동", "기본값 복원", "크기", "가로 위치", "세로 위치", "마우스로 이동...", "FPS 카운터를 끌어 놓은 뒤 완료를 누르세요", "배경 여백", "배경 불투명도", "켜짐", "꺼짐", "흰색", "보라색", "초록색", "노란색", "FPS에 따라", "왼쪽 위", "오른쪽 위", "왼쪽 아래", "오른쪽 아래", "완료"]),
    ("th", ["ตัวนับ FPS", "ค่าต่ำสุดและ 1% low", "พื้นหลัง", "สี", "ย้ายไปมุมถัดไป", "คืนค่าเริ่มต้น", "ขนาด", "ตำแหน่งแนวนอน", "ตำแหน่งแนวตั้ง", "ย้ายด้วยเมาส์...", "ลากตัวนับ FPS แล้วกดเสร็จสิ้น", "ขอบพื้นหลัง", "ความทึบพื้นหลัง", "เปิด", "ปิด", "ขาว", "ม่วง", "เขียว", "เหลือง", "ตาม FPS", "ซ้ายบน", "ขวาบน", "ซ้ายล่าง", "ขวาล่าง", "เสร็จสิ้น"]),
    ("vi", ["Bộ đếm FPS", "Tối thiểu và 1% low", "Nền", "Màu", "Chuyển sang góc tiếp theo", "Khôi phục mặc định", "Kích thước", "Vị trí ngang", "Vị trí dọc", "Di chuyển bằng chuột...", "Kéo bộ đếm FPS rồi nhấn Xong", "Lề nền", "Độ mờ nền", "Bật", "Tắt", "Trắng", "Tím", "Xanh lá", "Vàng", "Theo FPS", "Trên trái", "Trên phải", "Dưới trái", "Dưới phải", "Xong"]),
    ("id", ["Penghitung FPS", "Minimum dan 1% low", "Latar", "Warna", "Pindah ke sudut berikutnya", "Kembalikan bawaan", "Ukuran", "Posisi horizontal", "Posisi vertikal", "Pindahkan dengan mouse...", "Seret penghitung FPS, lalu tekan Selesai", "Margin latar", "Opasitas latar", "Nyala", "Mati", "Putih", "Ungu", "Hijau", "Kuning", "Menurut FPS", "Kiri atas", "Kanan atas", "Kiri bawah", "Kanan bawah", "Selesai"]),
];

fn strings() -> &'static [&'static str; 25] {
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

fn button(id: u32, label: String) -> Widget {
    Widget::Button(ButtonWidget { id, label })
}

fn slider(id: u32, label: String, steps: u32, position: u32) -> Widget {
    Widget::Slider(SliderWidget { id, label, steps, position })
}

impl Fps {
    fn scale(&self) -> f32 {
        self.scale as f32 / 100.0
    }

    fn slider_label(&self, id: u32) -> (String, u32, u32) {
        let t = strings();
        match id {
            S_SIZE => (
                format!("{}: {:.2}x", t[6], self.scale()),
                (SCALE_MAX - SCALE_MIN) / SCALE_STEP,
                (self.scale - SCALE_MIN) / SCALE_STEP,
            ),
            S_X => (format!("{}: {:.1}%", t[7], self.x as f32 / 10.0), 200, self.x / 5),
            S_Y => (format!("{}: {:.1}%", t[8], self.y as f32 / 10.0), 200, self.y / 5),
            S_PAD => (format!("{}: {:.1}", t[11], self.pad as f32 / 2.0), PAD_MAX, self.pad),
            _ => (format!("{}: {}%", t[12], self.opacity), 100, self.opacity),
        }
    }

    fn slider_widget(&self, id: u32) -> Widget {
        let (label, steps, position) = self.slider_label(id);
        slider(id, label, steps, position)
    }

    fn page(&self) {
        let t = strings();
        ui::set_settings_v2(&[
            button(SHOW, format!("{}: {}", t[0], on_off(t, self.show))),
            button(LOWS, format!("{}: {}", t[1], on_off(t, self.lows))),
            button(BACKGROUND, format!("{}: {}", t[2], on_off(t, self.background))),
            button(COLOR, format!("{}: {}", t[3], t[15 + self.color as usize])),
            button(CORNER, t[4].into()),
            self.slider_widget(S_SIZE),
            self.slider_widget(S_X),
            self.slider_widget(S_Y),
            self.slider_widget(S_PAD),
            self.slider_widget(S_OPACITY),
            button(RESET, strings()[5].into()),
        ]);
        ui::set_resets(&[SHOW, LOWS, BACKGROUND, COLOR, S_SIZE, S_X, S_Y, S_PAD, S_OPACITY]);
    }

    /// Shift + click: one setting back at its default.
    fn reset(&mut self, id: u32) {
        let mut d = Fps { ..*self };
        d.defaults();
        match id {
            SHOW => self.show = d.show,
            LOWS => self.lows = d.lows,
            BACKGROUND => self.background = d.background,
            COLOR => self.color = d.color,
            S_SIZE => self.scale = d.scale,
            S_X => self.x = d.x,
            S_Y => self.y = d.y,
            S_PAD => self.pad = d.pad,
            S_OPACITY => self.opacity = d.opacity,
            _ => return,
        }
        self.drawn = false;
        self.last = (0, 0, 0, 0, 0);
        self.save();
        self.page();
    }

    fn save(&self) {
        let text = format!(
            "show={}\nscale={}\nx={}\ny={}\npad={}\nopacity={}\nlows={}\nbackground={}\ncolor={}\n",
            u32::from(self.show),
            self.scale,
            self.x,
            self.y,
            self.pad,
            self.opacity,
            u32::from(self.lows),
            u32::from(self.background),
            self.color,
        );
        if storage::set(FILE, text.as_bytes()).is_ok() {
            for k in OLD_KEYS {
                storage::delete(k);
            }
        }
    }

    fn defaults(&mut self) {
        self.show = true;
        self.scale = 100;
        self.x = 320;
        self.y = 0;
        self.pad = 4;
        self.opacity = 50;
        self.lows = false;
        self.background = true;
        self.color = 4;
    }

    fn tint(&self, fps: u32) -> u32 {
        let rgb = COLORS[self.color as usize].1;
        let rgb = if rgb != 0 {
            rgb
        } else if fps >= 75 {
            0xB060FF
        } else if fps >= 60 {
            0x55FF55
        } else if fps >= 30 {
            0xFFFF55
        } else {
            0xFF5555
        };
        rgb
    }

    fn lines(&self, fps: u32, min: u32, low: u32) -> Vec<String> {
        let mut lines = vec![format!("{fps} fps")];
        if self.lows {
            lines.push(format!("min {min}"));
            lines.push(format!("1% low {low}"));
        }
        lines
    }

    fn draw(&mut self) {
        if !self.show {
            if self.drawn {
                render::set_hud(&[]);
                render::set_hud_drag(None);
                self.drawn = false;
            }
            return;
        }
        let stats = render::frame_stats();
        let (w, h) = render::gui_size();
        let key = (stats.fps, stats.min, stats.low, w as u32, h as u32);
        if self.drawn && key == self.last {
            return;
        }
        self.last = key;
        self.drawn = true;
        let lines = self.lines(stats.fps, stats.min, stats.low);
        let scale = self.scale();
        let (w, h) = (w / scale, h / scale);
        let pad = self.pad as f32 / 2.0;
        let line_h = 10.0;
        let text_w = lines.iter().map(|l| render::text_width(l)).fold(0.0, f32::max);
        let box_w = text_w + pad * 2.0;
        let box_h = lines.len() as f32 * line_h + pad * 2.0 - 1.0;
        let left = (w - box_w).max(0.0) * self.x as f32 / 1000.0;
        let top = (h - box_h).max(0.0) * self.y as f32 / 1000.0;
        let mut shapes = Vec::new();
        if self.background {
            shapes.push(Shape::Fill(FillShape {
                rect: Rect { x: left, y: top, w: box_w, h: box_h },
                color: (self.opacity * 255 / 100) << 24,
            }));
        }
        let color = self.tint(stats.fps);
        for (row, line) in lines.iter().enumerate() {
            shapes.push(Shape::Text(TextShape {
                text: line.clone(),
                at: Point { x: left + pad, y: top + pad + row as f32 * line_h },
                color: if row == 0 { color } else { 0xFFFFFF },
                centered: false,
                shadow: true,
            }));
        }
        render::set_hud_scale(scale);
        render::set_hud_always(true);
        render::set_hud(&shapes);
        render::set_hud_drag(Some(Rect { x: left, y: top, w: box_w, h: box_h }));
        self.area = (left, top, box_w, box_h, w, h);
    }

    fn dragged(&mut self) {
        let Some(d) = render::take_hud_drag() else { return };
        let (left, top, box_w, box_h, w, h) = self.area;
        let left = (left + d.dx).clamp(0.0, (w - box_w).max(0.0));
        let top = (top + d.dy).clamp(0.0, (h - box_h).max(0.0));
        let span_x = (w - box_w).max(1.0);
        let span_y = (h - box_h).max(1.0);
        self.x = ((left / span_x) * 1000.0).round().clamp(0.0, 1000.0) as u32;
        self.y = ((top / span_y) * 1000.0).round().clamp(0.0, 1000.0) as u32;
        self.drawn = false;
        if d.done {
            self.save();
            self.page();
        }
    }
}

impl Mod for Fps {
    fn init() -> Result<(Self, Registration), String> {
        let cfg = load_config();
        let txt = storage::get(FILE).is_some();
        let (xk, yk) = if txt { ("x", "y") } else { ("xt", "yt") };
        let mut fps = Fps {
            show: stored(&cfg, "show", 1, 0, 1) == 1,
            scale: stored(&cfg, "scale", 100, SCALE_MIN, SCALE_MAX) / SCALE_STEP * SCALE_STEP,
            x: stored(&cfg, xk, 320, 0, 1000) / 5 * 5,
            y: stored(&cfg, yk, 0, 0, 1000) / 5 * 5,
            pad: stored(&cfg, "pad", 4, 0, PAD_MAX),
            opacity: stored(&cfg, "opacity", 50, 0, 100),
            lows: stored(&cfg, "lows", 0, 0, 1) == 1,
            background: stored(&cfg, "background", 1, 0, 1) == 1,
            color: stored(&cfg, "color", 4, 0, COLORS.len() as u32 - 1),
            last: (0, 0, 0, 0, 0),
            drawn: false,
            area: (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
        };
        fps.scale = fps.scale.max(SCALE_MIN);
        fps.save();
        fps.page();
        Ok((fps, Registration::new().on_frame()))
    }

    fn frame(&mut self, _seconds: f32) {
        self.dragged();
        self.draw();
    }

    fn ui_select(&mut self, id: u32, row: u32) {
        if row == u32::MAX {
            self.reset(id);
            return;
        }
        let slid = match id {
            S_SIZE => {
                self.scale = SCALE_MIN + row * SCALE_STEP;
                true
            }
            S_X => {
                self.x = (row * 5).min(1000);
                true
            }
            S_Y => {
                self.y = (row * 5).min(1000);
                true
            }
            S_PAD => {
                self.pad = row.min(PAD_MAX);
                true
            }
            S_OPACITY => {
                self.opacity = row.min(100);
                true
            }
            _ => false,
        };
        if slid {
            let (label, _, position) = self.slider_label(id);
            ui::set_slider(id, position, &label);
            self.drawn = false;
            self.last = (0, 0, 0, 0, 0);
            self.save();
            return;
        }
        match id {
            SHOW => self.show = !self.show,
            CORNER => {
                let next = CORNERS
                    .iter()
                    .position(|c| c.1 == self.x && c.2 == self.y)
                    .map_or(0, |i| (i + 1) % CORNERS.len());
                self.x = CORNERS[next].1;
                self.y = CORNERS[next].2;
            }
            LOWS => self.lows = !self.lows,
            BACKGROUND => self.background = !self.background,
            COLOR => self.color = (self.color + 1) % COLORS.len() as u32,
            RESET => self.defaults(),
            _ => return,
        }
        self.drawn = false;
        self.last = (0, 0, 0, 0, 0);
        self.save();
        self.page();
    }

    fn ui_settings_opened(&mut self) {
        self.page();
    }
}

horizon_mod_sdk::export_mod!(Fps);
