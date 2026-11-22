//! Loads the wasm mods of a folder and draws one HUD frame with a stand-in game, printing what
//! the mods drew: `cargo run -p pojarush-wasm-mods --example hud_probe -- <mods dir>`.
use pojarush_wasm_mods::{View, WasmMods};
use std::path::PathBuf;

struct Log;
impl log::Log for Log {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }
    fn log(&self, r: &log::Record) {
        if r.level() <= log::Level::Info || r.target().starts_with("horizon_mod") {
            eprintln!("[{} {}] {}", r.level(), r.target(), r.args());
        }
    }
    fn flush(&self) {}
}

fn main() {
    log::set_logger(&Log).unwrap();
    log::set_max_level(log::LevelFilter::Debug);
    let dir = PathBuf::from(std::env::args().nth(1).expect("mods dir"));
    let mods = WasmMods::start(&dir, &dir.join("game"), &[]);
    let started = std::time::Instant::now();
    while !mods.is_running() && started.elapsed().as_secs() < 300 {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    for e in mods.entries() {
        println!("{} {} {:?}", e.id, e.version, e.problem);
    }
    let view = |down: bool| View {
        millis: started.elapsed().as_millis() as u64,
        playing: true,
        mappings: ["key.forward", "key.left", "key.back", "key.right", "key.jump", "key.attack", "key.use"]
            .iter()
            .map(|n| (*n, n[4..5].to_uppercase(), down && *n == "key.forward"))
            .collect(),
        corners_shown: [true; 4],
        settings_root: dir.join(".horizon-mod-data"),
        widths: std::sync::Arc::from(vec![6.0f32; 256]),
        language: "pt_br".into(),
        ..View::default()
    };
    mods.mouse_button_pre(view(true), 0, true);
    mods.render_frame_pre(view(true), std::time::Duration::from_millis(16), 0.5);
    let t = std::time::Instant::now();
    let (cmds, used) = mods.render_gui_post(view(true), (427.0, 240.0), [0.0; 4]);
    println!("frame in {:?}", t.elapsed());
    for c in cmds {
        println!("{c:?}");
    }
    println!("corners {used:?}");
    for e in mods.entries() {
        println!("settings {:?}", mods.settings_labels(&e.id, view(false)));
        mods.settings_click(&e.id, 0, view(false));
        println!("after click {:?}", mods.settings_labels(&e.id, view(false)));
    }
}
