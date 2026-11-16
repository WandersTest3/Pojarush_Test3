use horizon_mod_sdk::render::{self, FillShape, Point, Rect, Shape, TextShape};

pub struct Panel {
    pub x: f32,
    pub y: f32,
    pub scale: f32,
    pub padding: f32,
    pub background: u32,
    pub always: bool,
    pub draggable: bool,
    size: (f32, f32),
}

impl Default for Panel {
    fn default() -> Self {
        Self { x: 4.0, y: 4.0, scale: 1.0, padding: 2.0, background: 0x80000000, always: false, draggable: true, size: (0.0, 0.0) }
    }
}

impl Panel {
    pub fn lines(&mut self, lines: &[(String, u32)]) {
        let pad = self.padding;
        let w = lines.iter().map(|(t, _)| render::text_width(t)).fold(0.0, f32::max) + pad * 2.0;
        let h = lines.len() as f32 * 10.0 + pad * 2.0 - 1.0;
        let (gw, gh) = render::gui_size();
        let (gw, gh) = (gw / self.scale, gh / self.scale);
        self.x = self.x.clamp(0.0, (gw - w).max(0.0));
        self.y = self.y.clamp(0.0, (gh - h).max(0.0));
        self.size = (w, h);
        let mut shapes = Vec::with_capacity(lines.len() + 1);
        if self.background >> 24 != 0 {
            shapes.push(Shape::Fill(FillShape { rect: Rect { x: self.x, y: self.y, w, h }, color: self.background }));
        }
        for (i, (text, color)) in lines.iter().enumerate() {
            shapes.push(Shape::Text(TextShape {
                text: text.clone(),
                at: Point { x: self.x + pad, y: self.y + pad + i as f32 * 10.0 },
                color: *color,
                centered: false,
                shadow: true,
            }));
        }
        render::set_hud_scale(self.scale);
        render::set_hud_always(self.always);
        render::set_hud(&shapes);
        render::set_hud_drag(self.draggable.then_some(Rect { x: self.x, y: self.y, w, h }));
    }

    pub fn dragged(&mut self) -> Option<bool> {
        let d = render::take_hud_drag()?;
        self.x += d.dx;
        self.y += d.dy;
        Some(d.done)
    }

    pub fn clear(&mut self) {
        render::set_hud(&[]);
        render::set_hud_drag(None);
    }

    pub fn size(&self) -> (f32, f32) {
        self.size
    }
}
