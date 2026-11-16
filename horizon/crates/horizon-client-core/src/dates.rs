use chrono::Datelike;

pub fn today() -> (u32, u32) {
    if let Some((m, d)) = std::env::var("POJARUSH_DATE").ok().and_then(|v| {
        let (m, d) = v.trim().split_once('-')?;
        Some((m.parse::<u32>().ok()?, d.parse::<u32>().ok()?))
    }) {
        if (1..=12).contains(&m) && (1..=31).contains(&d) {
            return (m, d);
        }
    }
    let now = chrono::Local::now();
    (now.month(), now.day())
}
