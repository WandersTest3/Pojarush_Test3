use crate::config::{Config, Value};
use horizon_mod_sdk::ui::{self, ButtonWidget, SliderWidget, TextWidget, WidgetV2 as Widget};

pub const RESET: u32 = 9999;
const BASE: u32 = 1000;

pub fn page(config: &Config, labels: &dyn Fn(&str) -> String, on: &str, off: &str, reset: &str) {
    let mut widgets = Vec::new();
    for (i, e) in config.entries().iter().enumerate() {
        let id = BASE + i as u32;
        let label = labels(&e.key);
        match &e.value {
            Value::Bool(b) => widgets.push(Widget::Button(ButtonWidget { id, label: format!("{label}: {}", if *b { on } else { off }) })),
            Value::Int(v) => {
                let steps = (e.max - e.min).max(1.0) as u32;
                widgets.push(Widget::Slider(SliderWidget { id, label: format!("{label}: {v}"), steps, position: (*v as f64 - e.min).max(0.0) as u32 }))
            }
            Value::Float(v) => {
                let steps = (((e.max - e.min) * 100.0).round() as u32).clamp(1, 1000);
                let position = (((v - e.min) / (e.max - e.min).max(f64::EPSILON)) * steps as f64).round() as u32;
                widgets.push(Widget::Slider(SliderWidget { id, label: format!("{label}: {v:.2}"), steps, position }))
            }
            Value::Text(t) => widgets.push(Widget::Text(TextWidget { id, text: format!("{label}: {t}"), color: 0xFFFFFF })),
        }
    }
    widgets.push(Widget::Button(ButtonWidget { id: RESET, label: reset.to_owned() }));
    ui::set_settings_v2(&widgets);
}

pub fn select(config: &mut Config, id: u32, row: u32) -> bool {
    if id == RESET {
        config.reset();
        let _ = config.save();
        return true;
    }
    let Some(i) = id.checked_sub(BASE) else { return false };
    let Some(e) = config.entries().get(i as usize).cloned() else { return false };
    let value = match e.value {
        Value::Bool(b) => Value::Bool(!b),
        Value::Int(_) => Value::Int(e.min as i64 + row as i64),
        Value::Float(_) => {
            let steps = (((e.max - e.min) * 100.0).round() as u32).clamp(1, 1000);
            Value::Float(e.min + (e.max - e.min) * row as f64 / steps as f64)
        }
        Value::Text(_) => return false,
    };
    config.set(&e.key, value);
    let _ = config.save();
    true
}
