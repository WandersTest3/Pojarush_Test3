//! Loads the wasm mods of a folder and draws one HUD frame with a stand-in game, printing what
//! the mods drew: `cargo run -p pojarush-wasm-mods --example hud_probe -- <mods dir>`.
use pojarush_wasm_mods::{BlockDef, Crosshair, EntityView, Registries, View, WasmMods};
use std::collections::BTreeMap;
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
    let wheat = (0..8)
        .map(|a| BTreeMap::from([("age".to_owned(), a.to_string())]))
        .collect::<Vec<_>>();
    let registries = Registries {
        blocks: vec![
            BlockDef { name: "minecraft:air".into(), states: vec![BTreeMap::new()], default: 0, requires_tool: vec![false], has_block_entity: false },
            BlockDef { name: "minecraft:stone".into(), states: vec![BTreeMap::new()], default: 0, requires_tool: vec![true], has_block_entity: false },
            BlockDef { name: "minecraft:wheat".into(), states: wheat, default: 0, requires_tool: vec![false; 8], has_block_entity: false },
        ],
        items: ["minecraft:air", "minecraft:stone", "minecraft:wheat", "minecraft:wheat_seeds", "minecraft:diamond_pickaxe"].map(String::from).to_vec(),
        entity_types: ["minecraft:player", "minecraft:zombie", "minecraft:pig"].map(String::from).to_vec(),
    };
    let mods = WasmMods::start(&dir, &dir.join("game"), &dir.join("game/.horizon-wasm-cache"), &[], registries);
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
        crosshair: Some(if std::env::var_os("PROBE_ENTITY").is_some() {
            Crosshair::Entity { id: 7, point: [0.5, 65.0, 2.0] }
        } else {
            Crosshair::Block { pos: [1, 64, 2], block: "minecraft:wheat".into(), ordinal: 3, face: [0, 1, 0], point: [1.5, 65.0, 2.5] }
        }),
        entities: vec![
            EntityView { id: 1, kind: "minecraft:player".into(), name: "Steve".into(), health: Some((20.0, 20.0)), main_hand: Some(("minecraft:diamond_pickaxe".into(), 1)), ..EntityView::default() },
            EntityView { id: 7, kind: "minecraft:zombie".into(), name: "Zombie".into(), health: Some((13.0, 20.0)), ..EntityView::default() },
        ],
        local_player: Some(1),
        ..View::default()
    };
    mods.client_tick(view(true));
    let layers = mods.render_gui_layers(view(true), (427.0, 240.0));
    println!("layers drew {} commands", layers.len());
    for c in &layers {
        println!("  {c:?}");
    }
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
