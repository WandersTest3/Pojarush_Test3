pub fn color(name: &str) -> Option<u32> {
    Some(match name {
        "black" => 0x000000,
        "dark_blue" => 0x0000AA,
        "dark_green" => 0x00AA00,
        "dark_aqua" => 0x00AAAA,
        "dark_red" => 0xAA0000,
        "dark_purple" => 0xAA00AA,
        "gold" => 0xFFAA00,
        "gray" => 0xAAAAAA,
        "dark_gray" => 0x555555,
        "blue" => 0x5555FF,
        "green" => 0x55FF55,
        "aqua" => 0x55FFFF,
        "red" => 0xFF5555,
        "light_purple" => 0xFF55FF,
        "yellow" => 0xFFFF55,
        "white" => 0xFFFFFF,
        "pojarush" => 0x57197F,
        _ => return u32::from_str_radix(name.trim_start_matches('#'), 16).ok().filter(|_| name.starts_with('#')),
    })
}

pub fn placeholders(template: &str, values: &[(&str, String)]) -> String {
    let mut out = template.to_owned();
    for (k, v) in values {
        out = out.replace(&format!("%{k}%"), v);
    }
    out
}

pub fn wrap(text: &str, max: f32, width: impl Fn(&str) -> f32) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let next = if line.is_empty() { word.to_owned() } else { format!("{line} {word}") };
            if !line.is_empty() && width(&next) > max {
                lines.push(std::mem::take(&mut line));
                line = word.to_owned();
            } else {
                line = next;
            }
        }
        lines.push(line);
    }
    lines
}

pub fn lerp_color(a: u32, b: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let ch = |s: u32| {
        let x = ((a >> s) & 0xFF) as f32;
        let y = ((b >> s) & 0xFF) as f32;
        ((x + (y - x) * t).round() as u32) << s
    };
    ch(16) | ch(8) | ch(0)
}
