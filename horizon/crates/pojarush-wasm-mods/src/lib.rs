//! Horizon's wasm mods (`.hmod` with api "0.2" and a `[wasm]` table) in Pojarush.
//!
//! The vendored `horizon-mod-wasm` (LGPL-3.0-or-later, `vendor/horizon-wasm-mods`) links each
//! package against `horizon-wasm-std.wasm` and turns it into a `mod_api::ModEntry`; this crate
//! loads them, installs them in the vendored dispatch list and lends the game to their hooks.
//! Guests need a deep stack, so every call runs on one worker thread of guest size: the game
//! hands it a [`View`] of what mods may read and gets back what they drew ([`DrawCmd`]). The
//! host serves the HUD, key, click and settings methods HUD mods use; any other host method a
//! mod calls stops that call with "unprovided" instead of crashing the game.
use mod_api::{
    ClientHost, ConfigEntry, ConfigEntryKind, ConfigFileError, ConfigFiles, ConfigPath, ConfigStat, Corner,
    Corners, DrawContext, Host, KeyMapping, SettingKind, SettingSpec, SettingValue, Text,
    TextFormError,
};
use std::collections::{BTreeMap, HashMap};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, mpsc};
use std::time::Duration;

/// The wasm side of Horizon's standard library every package links against, built from the
/// vendored sources with the toolchain the packages name.
pub static STD_MEMBER: &[u8] =
    include_bytes!("../../../vendor/horizon-wasm-mods/horizon-wasm-std.wasm");

/// What mods may read of the game during one call, copied by the game.
#[derive(Clone, Default)]
pub struct View {
    /// Milliseconds on a monotonic clock.
    pub millis: u64,
    /// The player is in a world with no screen open.
    pub playing: bool,
    /// Vanilla mappings (`key.forward`): the bound key's label and whether it is held.
    pub mappings: Vec<(&'static str, String, bool)>,
    /// Whether a mod may draw in each corner (top left, top right, bottom left, bottom right).
    pub corners_shown: [bool; 4],
    /// `horizon-mods/.horizon-mod-data`: one folder of settings per mod.
    pub settings_root: PathBuf,
    /// The ids of the mods the player turned off.
    pub off: Vec<String>,
    /// The cursor in GUI pixels.
    pub cursor: [f32; 2],
    /// The HUD font's advance per character from U+0000.
    pub widths: Arc<[f32]>,
    /// The selected language (`pt_br`).
    pub language: String,
    /// The game's text for a translation key.
    pub translate: Option<fn(&str) -> Option<String>>,
    /// What the crosshair is on.
    pub crosshair: Option<Crosshair>,
    /// The entities mods may read (the crosshair's and the local player).
    pub entities: Vec<EntityView>,
    /// The local player's entity id among `entities`.
    pub local_player: Option<u32>,
    /// Keys pressed since the last tick, as `options.txt` names them (`key.keyboard.k`).
    pub pressed: Vec<String>,
    /// The game is paused.
    pub paused: bool,
}

/// What the game's registries hold, handed over once when the mods start, so mods can name
/// blocks, items and entity types (`BlockKey`, `ItemKey`, `EntityTypeKey`).
#[derive(Clone, Default)]
pub struct Registries {
    /// Every block in the game's state order.
    pub blocks: Vec<BlockDef>,
    /// Every item id.
    pub items: Vec<String>,
    /// Every entity type id.
    pub entity_types: Vec<String>,
}

/// One block: its states' properties in the game's order, the default among them.
#[derive(Clone, Default)]
pub struct BlockDef {
    pub name: String,
    pub states: Vec<BTreeMap<String, String>>,
    pub default: u16,
    /// Per state: whether it drops nothing without the right tool.
    pub requires_tool: Vec<bool>,
    pub has_block_entity: bool,
}

/// What the crosshair is on this frame.
#[derive(Clone, Debug)]
pub enum Crosshair {
    Block { pos: [i32; 3], block: String, ordinal: u16, face: [i32; 3], point: [f64; 3] },
    Entity { id: u32, point: [f64; 3] },
}

/// An entity mods may read: the one under the crosshair and the local player.
#[derive(Clone, Debug, Default)]
pub struct EntityView {
    pub id: u32,
    pub kind: String,
    pub name: String,
    pub position: [f64; 3],
    pub eye: [f64; 3],
    pub uuid: u128,
    /// Health and maximum health, for a living entity.
    pub health: Option<(f32, f32)>,
    /// The main hand's item id and count.
    pub main_hand: Option<(String, u32)>,
}

/// What a HUD hook drew, in GUI pixels, for the game to draw in order.
#[derive(Clone, Debug, PartialEq)]
pub enum DrawCmd {
    Fill { rect: [f32; 4], argb: u32 },
    Gradient { rect: [f32; 4], top: u32, bottom: u32 },
    Text { text: String, at: [f32; 2], argb: u32, shadow: bool, scale: f32 },
    Item { id: String, at: [f32; 2], size: f32 },
}

/// One wasm package found in the mods folder, for the Mods page.
#[derive(Clone, Debug)]
pub struct Entry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub authors: Vec<String>,
    pub license: String,
    pub homepage: Option<String>,
    pub path: PathBuf,
    /// Why it isn't running, if it isn't.
    pub problem: Option<String>,
}

type Job = Box<dyn FnOnce(&mut Inner) + Send>;

/// The wasm mods of this run, served by their guest thread.
pub struct WasmMods {
    jobs: Option<mpsc::Sender<Job>>,
    entries: Arc<Mutex<Vec<Entry>>>,
    ready: Arc<AtomicBool>,
}

impl Default for WasmMods {
    fn default() -> Self {
        Self {
            jobs: None,
            entries: Arc::default(),
            ready: Arc::default(),
        }
    }
}

const STARTING: &str = "Starting…";

impl WasmMods {
    /// Lists the `.hmod` files in `mods_dir` that carry a `[wasm]` table and starts the guest
    /// thread that compiles and installs the ones not in `off` (compiled modules are kept in
    /// `cache_dir`, a folder only this app may write, so later starts are quick). Hooks do
    /// nothing until it is done. Once per process.
    pub fn start(
        mods_dir: &Path,
        game_dir: &Path,
        cache_dir: &Path,
        off: &[String],
        registries: Registries,
    ) -> Self {
        let mut entries = Vec::new();
        let mut paths: Vec<PathBuf> = std::fs::read_dir(mods_dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "hmod"))
            .collect();
        paths.sort();
        for path in paths {
            let Ok(package) = horizon_mod_package::Package::read(&path) else {
                continue;
            };
            if package.manifest.wasm.is_none() {
                continue;
            }
            let m = package.manifest;
            entries.push(Entry {
                problem: Some(if off.contains(&m.id) { "Off".into() } else { STARTING.into() }),
                id: m.id,
                name: m.name,
                version: m.version,
                description: m.description,
                authors: m.authors,
                license: m.license,
                homepage: m.homepage,
                path,
            });
        }
        let shared = Arc::new(Mutex::new(entries.clone()));
        let ready = Arc::new(AtomicBool::new(false));
        if entries.iter().all(|e| e.problem.as_deref() != Some(STARTING)) {
            return Self {
                jobs: None,
                entries: shared,
                ready,
            };
        }
        let (tx, rx) = mpsc::channel::<Job>();
        let game_dir = game_dir.to_owned();
        let cache_dir = cache_dir.to_owned();
        let (shared2, ready2) = (shared.clone(), ready.clone());
        let spawned = horizon_guest_stack::builder()
            .name("wasm-mods".into())
            .spawn(move || {
                let mut inner = Inner::load(entries, &game_dir, &cache_dir, registries);
                if let Ok(mut s) = shared2.lock() {
                    *s = inner.entries.clone();
                }
                ready2.store(inner.running, Ordering::Release);
                for job in rx {
                    job(&mut inner);
                }
            });
        match spawned {
            Ok(_) => Self {
                jobs: Some(tx),
                entries: shared,
                ready,
            },
            Err(e) => {
                if let Ok(mut s) = shared.lock() {
                    for entry in s.iter_mut().filter(|e| e.problem.as_deref() == Some(STARTING)) {
                        entry.problem = Some(format!("no guest thread: {e}"));
                    }
                }
                Self {
                    jobs: None,
                    entries: shared,
                    ready,
                }
            }
        }
    }

    /// The packages found, with why each isn't running.
    pub fn entries(&self) -> Vec<Entry> {
        self.entries.lock().map(|e| e.clone()).unwrap_or_default()
    }

    /// Whether at least one mod runs.
    pub fn is_running(&self) -> bool {
        self.ready.load(Ordering::Acquire)
    }

    /// Runs `f` on the guest thread and waits for it; None while the mods start or after the
    /// thread is gone.
    fn call<R: Send + 'static>(&self, f: impl FnOnce(&mut Inner) -> R + Send + 'static) -> Option<R> {
        if !self.is_running() {
            return None;
        }
        let (tx, rx) = mpsc::sync_channel(1);
        self.jobs
            .as_ref()?
            .send(Box::new(move |inner| {
                let _ = tx.send(f(inner));
            }))
            .ok()?;
        rx.recv().ok()
    }

    /// The HUD, after the game's own overlays: each mod places its box below what the corners
    /// hold (`used`, per corner) and draws it. The commands and the corners' new heights.
    pub fn render_gui_post(&self, view: View, size: (f32, f32), used: [f32; 4]) -> (Vec<DrawCmd>, [f32; 4]) {
        self.call(move |inner| inner.render_gui_post(&view, size, used))
            .unwrap_or((Vec::new(), used))
    }

    /// The GUI layers mods registered (`register_gui_layers`), drawn after the HUD.
    pub fn render_gui_layers(&self, view: View, size: (f32, f32)) -> Vec<DrawCmd> {
        self.call(move |inner| inner.render_gui_layers(&view, size)).unwrap_or_default()
    }

    /// Once a client tick, before it runs.
    pub fn client_tick(&self, view: View) {
        self.call(move |inner| inner.client_tick(&view));
    }

    /// Every frame, before the world draws.
    pub fn render_frame_pre(&self, view: View, elapsed: Duration, partial_tick: f32) {
        self.call(move |inner| inner.render_frame_pre(&view, elapsed, partial_tick));
    }

    /// A mouse button of the window (0 left, 1 right, 2 middle). True when a mod canceled it.
    pub fn mouse_button_pre(&self, view: View, button: i32, pressed: bool) -> bool {
        self.call(move |inner| inner.mouse_button_pre(&view, button, pressed))
            .unwrap_or(false)
    }

    /// After the resource packs (and the language) reload.
    pub fn resources_reloaded(&self, view: View) {
        self.call(move |inner| inner.resources_reloaded(&view));
    }

    /// The settings page's buttons of mod `id`: one per registered setting, "Label: value".
    /// None for a mod not running.
    pub fn settings_labels(&self, id: &str, view: View) -> Option<Vec<String>> {
        let id = id.to_owned();
        self.call(move |inner| inner.settings_labels(&id, &view)).flatten()
    }

    /// A click on button `index` of mod `id`'s page: toggles flip, choices and colours move to
    /// the next option, ranges to the next step (back to the start after the end).
    pub fn settings_click(&self, id: &str, index: usize, view: View) {
        let id = id.to_owned();
        self.call(move |inner| inner.settings_click(&id, index, &view));
    }
}

/// The guest thread's side: the loaded mods and their translations.
struct Inner {
    entries: Vec<Entry>,
    running: bool,
    /// The GUI layers mods registered, in order.
    layers: Vec<String>,
    /// Clicks of each mod key mapping (`mod id`, `key id`) not consumed yet.
    clicks: HashMap<(String, String), u32>,
    /// Translations from the packages' `assets/<ns>/lang/<code>.json`, by language.
    lang: HashMap<String, HashMap<String, String>>,
}

static FILES_ROOT: OnceLock<PathBuf> = OnceLock::new();

impl Inner {
    fn load(mut entries: Vec<Entry>, game_dir: &Path, cache_dir: &Path, registries: Registries) -> Self {
        let _ = FILES_ROOT.set(game_dir.join("horizon-mod-data"));
        let mut lang: HashMap<String, HashMap<String, String>> = HashMap::new();
        let mut packages = Vec::new();
        if let Err(why) = horizon_guest_stack::check_current() {
            for e in entries.iter_mut().filter(|e| e.problem.as_deref() == Some(STARTING)) {
                e.problem = Some(why.to_owned());
            }
            return Self { entries, running: false, layers: Vec::new(), clicks: HashMap::new(), lang };
        }
        for entry in entries.iter_mut().filter(|e| e.problem.as_deref() == Some(STARTING)) {
            read_lang(&entry.path, &mut lang);
            let module = horizon_mod_package::Package::read(&entry.path)
                .map_err(|e| e.to_string())
                .and_then(|p| Ok((p.wasm(&entry.path)?, p.manifest)));
            match module {
                Ok((module, manifest)) => {
                    entry.problem = None;
                    packages.push(horizon_mod_wasm::WasmPackage { manifest, module });
                }
                Err(why) => entry.problem = Some(why),
            }
        }
        let mut running = false;
        let mut layers = Vec::new();
        if !packages.is_empty() {
            let started = std::time::Instant::now();
            let loaded = horizon_mod_wasm::load_cached(
                STD_MEMBER,
                packages,
                cache_dir,
            );
            let mut refused: Vec<(String, String)> =
                loaded.refused.iter().map(|(id, why)| (id.clone(), why.to_string())).collect();
            if let Err(more) = loaded.initialize(&[]) {
                refused.extend(more.into_iter().map(|(id, why)| (id, why.to_string())));
            }
            for (id, why) in &refused {
                log::warn!("wasm mod {id} refused: {why}");
                if let Some(e) = entries.iter_mut().find(|e| &e.id == id) {
                    e.problem = Some(why.clone());
                }
            }
            running = !loaded.mods.is_empty();
            if running {
                mod_api::__private::install(loaded.mods.into());
                running = start_phases(&mut entries, &registries);
                if running {
                    let context = <mod_api::RegistrationContext as mod_api::__private::game::RegistrationContextNew>::new();
                    let mut found = Vec::new();
                    guarded("register_gui_layers", || {
                        mod_api::dispatch_register_gui_layers(&context, &mut found)
                    });
                    layers = found.into_iter().map(|l| l.id).collect();
                }
            }
            log::info!(
                "wasm mods: {} running, {} refused, in {:?}",
                entries.iter().filter(|e| e.problem.is_none()).count(),
                refused.len(),
                started.elapsed()
            );
        }
        Self { entries, running, layers, clicks: HashMap::new(), lang }
    }

    fn host<'a>(&'a self, view: &'a View) -> PojarushHost<'a> {
        if let Some(t) = view.translate {
            let _ = TRANSLATE.set(t);
        }
        PojarushHost { view, lang: &self.lang, clicks: std::cell::RefCell::new(HashMap::new()) }
    }

    fn render_gui_post(&self, view: &View, size: (f32, f32), used: [f32; 4]) -> (Vec<DrawCmd>, [f32; 4]) {
        let mut corners = Corners::from_parts(size.0, size.1, used);
        let mut out = Recorder::new(view.widths.clone(), size);
        let host = self.host(view);
        guarded("render_gui_post", || {
            mod_api::dispatch_render_gui_post(&host, &mut out, &mut corners)
        });
        (out.cmds, corners.parts().2)
    }

    fn render_gui_layers(&self, view: &View, size: (f32, f32)) -> Vec<DrawCmd> {
        let mut out = Recorder::new(view.widths.clone(), size);
        let host = self.host(view);
        for layer in &self.layers {
            guarded("gui_layer_draw", || mod_api::dispatch_gui_layer_draw(&host, layer, &mut out));
            out.reset_pose();
        }
        out.cmds
    }

    fn client_tick(&mut self, view: &View) {
        for (owner, key) in mod_api::__private::key_mappings() {
            if view.pressed.iter().any(|p| *p == key.key) {
                *self.clicks.entry(((*owner).to_owned(), key.id.clone())).or_default() += 1;
            }
        }
        if let Some(t) = view.translate {
            let _ = TRANSLATE.set(t);
        }
        let clicks = std::mem::take(&mut self.clicks);
        let mut host = PojarushHost { view, lang: &self.lang, clicks: std::cell::RefCell::new(clicks) };
        guarded("client_tick_pre", || mod_api::dispatch_client_tick_pre(&mut host));
        self.clicks = host.clicks.into_inner();
    }

    fn render_frame_pre(&self, view: &View, elapsed: Duration, partial_tick: f32) {
        let mut host = self.host(view);
        guarded("render_frame_pre", || {
            mod_api::dispatch_render_frame_pre(&mut host, elapsed, partial_tick, partial_tick)
        });
    }

    fn mouse_button_pre(&self, view: &View, button: i32, pressed: bool) -> bool {
        let mut canceled = false;
        let mut host = self.host(view);
        guarded("mouse_button_pre", || {
            mod_api::dispatch_mouse_button_pre(&mut host, button, pressed, 0, &mut canceled)
        });
        canceled
    }

    fn resources_reloaded(&self, view: &View) {
        let mut host = self.host(view);
        guarded("client_resources_reloaded", || {
            mod_api::dispatch_client_resources_reloaded(&mut host)
        });
    }

    fn settings_labels(&self, id: &str, view: &View) -> Option<Vec<String>> {
        self.entries.iter().find(|e| e.id == id && e.problem.is_none())?;
        let host = self.host(view);
        let tr = |key: &str| host.tr(key).unwrap_or_else(|| key.to_owned());
        let dir = view.settings_root.join(id);
        Some(
            settings_of(id)
                .into_iter()
                .map(|spec| {
                    let value = spec.read(read_setting(&dir, &spec.key).as_deref());
                    let shown = match (&spec.kind, value) {
                        (_, SettingValue::Toggle(on)) => {
                            tr(if on { "options.on" } else { "options.off" })
                        }
                        (SettingKind::Choice { options, .. }, SettingValue::Choice(i)) => {
                            options.get(i).map(|o| tr(o)).unwrap_or_default()
                        }
                        (_, SettingValue::Range(v)) => format!("{v:.2}"),
                        (_, SettingValue::Color(rgb)) => format!("#{rgb:06X}"),
                        _ => String::new(),
                    };
                    format!("{}: {shown}", tr(&spec.label))
                })
                .collect(),
        )
    }

    fn settings_click(&self, id: &str, index: usize, view: &View) {
        let Some(spec) = settings_of(id).into_iter().nth(index) else {
            return;
        };
        let dir = view.settings_root.join(id);
        let next = match (&spec.kind, spec.read(read_setting(&dir, &spec.key).as_deref())) {
            (_, SettingValue::Toggle(on)) => SettingValue::Toggle(!on),
            (SettingKind::Choice { options, .. }, SettingValue::Choice(i)) => {
                SettingValue::Choice((i + 1) % options.len().max(1))
            }
            (SettingKind::Range { min, max, step, .. }, SettingValue::Range(v)) => {
                let n = v + step;
                SettingValue::Range(if n > *max + step * 0.01 { *min } else { n.min(*max) })
            }
            (_, other) => other,
        };
        if let Some(text) = spec.store(next) {
            store_setting(&dir, &spec.key, &text);
            let mut host = self.host(view);
            guarded("setting_changed", || {
                mod_api::dispatch_setting_changed(&mut host, id, &spec.key)
            });
        }
    }
}

/// The translations of a package: `assets/<namespace>/lang/<code>.json`, flat string maps.
fn read_lang(path: &Path, lang: &mut HashMap<String, HashMap<String, String>>) {
    let Ok(file) = std::fs::File::open(path) else {
        return;
    };
    let Ok(mut zip) = zip::ZipArchive::new(file) else {
        return;
    };
    let names: Vec<String> = zip
        .file_names()
        .filter_map(|n| n.ok().map(|n| n.into_owned()))
        .filter(|n| n.starts_with("assets/") && n.contains("/lang/") && n.ends_with(".json"))
        .collect();
    for name in names {
        let Some(code) = name.rsplit('/').next().and_then(|f| f.strip_suffix(".json")) else {
            continue;
        };
        let mut text = String::new();
        let read = zip
            .by_name(&name)
            .ok()
            .and_then(|f| f.take(1 << 20).read_to_string(&mut text).ok());
        if read.is_none() {
            continue;
        }
        if let Ok(serde_json::Value::Object(map)) = serde_json::from_str(&text) {
            let table = lang.entry(code.to_lowercase()).or_default();
            for (k, v) in map {
                if let serde_json::Value::String(v) = v {
                    table.insert(k, v);
                }
            }
        }
    }
}

/// The phases a mod meets as the game starts: registration (its settings among what it
/// registers), setup, then the world phase every HUD hook runs in.
fn start_phases(entries: &mut [Entry], registries: &Registries) -> bool {
    use mod_api::__private::{ModPhase, PhaseOwner, RuleSchema};
    let phase = PhaseOwner::new(ModPhase::Packages);
    let step = |to| phase.advance(to).map_err(|e| e.to_string());
    let result = step(ModPhase::Registration)
        .and_then(|()| {
            mod_api::__private::install_parts(
                mod_api::__private::mods(),
                &RuleSchema {
                    category: &|_| false,
                    position: &|_| None,
                    rule_count: 0,
                },
                |_| false,
            )
            .map_err(|refused| refused.join("; "))
        })
        .and_then(|()| resolve_registries(registries))
        .and_then(|()| step(ModPhase::Setup))
        .and_then(|()| step(ModPhase::Config))
        .and_then(|()| step(ModPhase::World));
    match result {
        Ok(()) => {
            std::mem::forget(phase);
            true
        }
        Err(why) => {
            log::warn!("wasm mods stopped at registration: {why}");
            for e in entries.iter_mut().filter(|e| e.problem.is_none()) {
                e.problem = Some(why.clone());
            }
            false
        }
    }
}

/// The block tables mods read through `BlockState` and `HostBlockStates`.
struct Tables {
    definitions: HashMap<String, Vec<mod_api::StateProperty>>,
    requires_tool: HashMap<String, Vec<bool>>,
    has_entity: HashMap<String, bool>,
}
static TABLES: OnceLock<Tables> = OnceLock::new();

/// Completes the mods' requested keys with the game's registries.
fn resolve_registries(r: &Registries) -> Result<(), String> {
    let join = |e: Vec<String>| e.join("; ");
    let blocks: HashMap<&str, &BlockDef> = r.blocks.iter().map(|b| (b.name.as_str(), b)).collect();
    mod_api::__private::resolve_block_keys(r.blocks.iter().map(|b| b.name.as_str()), |name| {
        let def = blocks.get(name).ok_or_else(|| format!("no block {name}"))?;
        let states: Vec<&BTreeMap<String, String>> = def.states.iter().collect();
        horizon_mod_runtime::block_definitions::BlockDefinition::new(&states, def.default)
            .map_err(|e| format!("block {name}: {e:?}"))
    })
    .map_err(join)?;
    mod_api::__private::resolve_items(r.items.iter().map(String::as_str))
        .map_err(|e| e.into_iter().map(|k| format!("mod {} requested unknown item {}", k.mod_id, k.id)).collect::<Vec<_>>().join("; "))?;
    mod_api::__private::resolve_entity_types(r.entity_types.iter().map(String::as_str)).map_err(join)?;
    let mut tables = Tables {
        definitions: HashMap::new(),
        requires_tool: HashMap::new(),
        has_entity: HashMap::new(),
    };
    for b in &r.blocks {
        let mut props: Vec<mod_api::StateProperty> = Vec::new();
        for state in &b.states {
            for (k, v) in state {
                match props.iter_mut().find(|p| &p.name == k) {
                    Some(p) if !p.values.contains(v) => p.values.push(v.clone()),
                    Some(_) => {}
                    None => props.push(mod_api::StateProperty::new(k.clone(), vec![v.clone()])),
                }
            }
        }
        tables.definitions.insert(b.name.clone(), props);
        tables.requires_tool.insert(b.name.clone(), b.requires_tool.clone());
        tables.has_entity.insert(b.name.clone(), b.has_block_entity);
    }
    let _ = TABLES.set(tables);
    Ok(())
}

fn settings_of(id: &str) -> Vec<SettingSpec> {
    mod_api::__private::settings()
        .iter()
        .filter(|(owner, _)| *owner == id)
        .map(|(_, spec)| spec.clone())
        .collect()
}

/// A panic in a hook (an unprovided host method among them) stops that call, not the game.
fn guarded(hook: &str, f: impl FnOnce()) {
    if let Err(panic) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        static SEEN: Mutex<Vec<String>> = Mutex::new(Vec::new());
        let why = panic
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| panic.downcast_ref::<&str>().map(|s| (*s).to_owned()))
            .unwrap_or_default();
        let line = format!("{hook}: {why}");
        if let Ok(mut seen) = SEEN.lock()
            && !seen.contains(&line)
        {
            log::warn!("wasm mod hook stopped: {line}");
            seen.push(line);
        }
    }
}

fn read_setting(dir: &Path, key: &str) -> Option<String> {
    if !key_is_valid(key) {
        return None;
    }
    std::fs::read_to_string(dir.join(key)).ok()
}

fn store_setting(dir: &Path, key: &str, value: &str) -> bool {
    key_is_valid(key)
        && std::fs::create_dir_all(dir).is_ok()
        && std::fs::write(dir.join(key), value).is_ok()
}

/// A setting key as `SettingSpec::check` takes it: a file name in the mod's folder.
fn key_is_valid(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 64
        && !key.starts_with('.')
        && key.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_-.".contains(&b))
}

fn calling_mod() -> Option<&'static str> {
    mod_api::content::crash::current_native_mod()
}

struct PojarushHost<'a> {
    view: &'a View,
    lang: &'a HashMap<String, HashMap<String, String>>,
    clicks: std::cell::RefCell<HashMap<(String, String), u32>>,
}

impl PojarushHost<'_> {
    /// The game's text, else the packages' own for the language, else their English.
    fn tr(&self, key: &str) -> Option<String> {
        self.view
            .translate
            .and_then(|t| t(key))
            .or_else(|| self.lang.get(&self.view.language)?.get(key).cloned())
            .or_else(|| self.lang.get("en_us")?.get(key).cloned())
    }
    fn own_setting(&self, key: &str) -> Option<SettingValue> {
        let id = calling_mod()?;
        let spec = mod_api::__private::setting(id, key)?;
        Some(spec.read(read_setting(&self.view.settings_root.join(id), key).as_deref()))
    }
}

impl Host for PojarushHost<'_> {
    fn backend(&self) -> Option<mod_api::BackendRef<'_>> {
        Some(mod_api::__private::backend_ref(self))
    }
}
mod_api::impl_host_areas!(PojarushHost<'_>, except(config, entities, living, items, keys, block_states, payload_tokens));

impl mod_api::prelude::PayloadTokensHost for PojarushHost<'_> {
    /// No wasm mod's server half runs in Pojarush yet: an optional channel is absent.
    fn check_payload_token(&self, token: &mod_api::PayloadToken) -> Result<(), mod_api::PayloadSendError> {
        Err(mod_api::PayloadSendError::UnsupportedPeer(token.id.clone()))
    }
}
impl ClientHost for PojarushHost<'_> {}
mod_api::impl_client_host_areas!(PojarushHost<'_>, except(client_input, client_view, config, menus, payloads));

impl mod_api::prelude::PayloadsClientHost for PojarushHost<'_> {
    /// Pojarush's integrated server runs no wasm mod's server half yet: every optional
    /// channel is absent, as on a server without the mod.
    fn send_payload_to_server(
        &mut self,
        id: &mod_api::PayloadId,
        _: mod_api::PayloadBytes,
    ) -> Result<mod_api::PayloadAccepted, mod_api::PayloadSendError> {
        Err(mod_api::PayloadSendError::UnsupportedPeer(id.clone()))
    }
}

/// The world epoch of the entity handles this host hands out.
const WORLD_EPOCH: u64 = 1;

impl PojarushHost<'_> {
    fn entity_view(&self, entity: mod_api::EntityRef) -> Option<&EntityView> {
        self.view.entities.iter().find(|e| e.id == entity.id())
    }
    fn block_name(&self, block: mod_api::BlockKey) -> Option<&'static str> {
        use mod_api::BlockMetadata;
        mod_api::__private::block_definitions()?.name(block).ok()
    }
}

impl mod_api::Backend for PojarushHost<'_> {}
mod_api::impl_backend_areas!(PojarushHost<'_>, except(keys, items, entities, living, block_states));

impl mod_api::prelude::KeysBackend for PojarushHost<'_> {
    fn block_key_id(&self, key: mod_api::BlockKey) -> String {
        self.block_name(key).unwrap_or("minecraft:air").to_owned()
    }
    fn item_key_id(&self, key: mod_api::ItemKey) -> String {
        mod_api::__private::item_key_id(key).map(|s| s.to_string()).unwrap_or_else(|| "minecraft:air".into())
    }
    fn entity_type_key_id(&self, key: mod_api::EntityTypeKey) -> String {
        mod_api::__private::entity_type_key_id(key).unwrap_or_else(|| "minecraft:pig".into())
    }
}
impl mod_api::prelude::KeysHost for PojarushHost<'_> {}

impl mod_api::prelude::ItemsHost for PojarushHost<'_> {
    fn item_key(&self, id: &mod_api::Identifier) -> Option<mod_api::ItemKey> {
        mod_api::__private::item_key_by_identifier(id)
    }
}
impl mod_api::prelude::ItemsBackend for PojarushHost<'_> {}

impl mod_api::prelude::EntitiesHost for PojarushHost<'_> {
    fn entity(&self, id: u32) -> Option<mod_api::EntityRef> {
        self.view
            .entities
            .iter()
            .any(|e| e.id == id)
            .then(|| mod_api::EntityRef::new(mod_api::WorldEpoch::new(WORLD_EPOCH), id))
    }
}
impl mod_api::prelude::EntitiesBackend for PojarushHost<'_> {
    fn entity_name(&self, entity: mod_api::EntityRef) -> Option<String> {
        Some(self.entity_view(entity)?.name.clone())
    }
    fn entity_position(&self, entity: mod_api::EntityRef) -> Option<[f64; 3]> {
        Some(self.entity_view(entity)?.position)
    }
    fn entity_eye_position(&self, entity: mod_api::EntityRef) -> Option<[f64; 3]> {
        Some(self.entity_view(entity)?.eye)
    }
    fn entity_kind(&self, entity: mod_api::EntityRef) -> Option<mod_api::EntityTypeKey> {
        mod_api::__private::resolved_entity_type_key(&self.entity_view(entity)?.kind)
    }
}
impl mod_api::prelude::EntitiesBackendMut for PojarushHost<'_> {
    fn entity_uuid(&mut self, entity: mod_api::EntityRef) -> Option<u128> {
        Some(self.entity_view(entity)?.uuid)
    }
}

impl mod_api::prelude::LivingHost for PojarushHost<'_> {
    fn living(&self, entity: mod_api::EntityRef) -> Option<mod_api::LivingRef> {
        self.entity_view(entity)?.health?;
        Some(mod_api::LivingRef::new(entity))
    }
}
impl mod_api::prelude::LivingClientBackend for PojarushHost<'_> {}
impl mod_api::prelude::LivingBackend for PojarushHost<'_> {
    fn living_health(&self, living: mod_api::LivingRef) -> Option<f32> {
        Some(self.entity_view(living.entity())?.health?.0)
    }
    fn living_max_health(&self, living: mod_api::LivingRef) -> Option<f32> {
        Some(self.entity_view(living.entity())?.health?.1)
    }
    fn living_hand_item(&self, living: mod_api::LivingRef, hand: mod_api::Hand) -> Option<mod_api::ItemStack> {
        if hand != mod_api::Hand::Main {
            return None;
        }
        let (id, count) = self.entity_view(living.entity())?.main_hand.as_ref()?;
        mod_api::ItemStack::new(mod_api::__private::item_key(id)?, *count)
    }
}

impl mod_api::prelude::BlockStatesHost for PojarushHost<'_> {
    fn block_item(&self, block: mod_api::BlockKey) -> Option<mod_api::ItemKey> {
        mod_api::__private::item_key(self.block_name(block)?)
    }
    fn item_block(&self, item: mod_api::ItemKey) -> Option<mod_api::BlockKey> {
        mod_api::__private::block_key(&mod_api::__private::item_key_id(item)?.to_string())
    }
}
impl mod_api::prelude::BlockStatesBackend for PojarushHost<'_> {
    fn block_metadata_name(&self, block: mod_api::BlockKey) -> Result<&str, mod_api::BlockStateError> {
        self.block_name(block).ok_or(mod_api::BlockStateError::UnknownBlock)
    }
    fn block_metadata_property(
        &self,
        state: mod_api::BlockStateRecord,
        name: &str,
    ) -> Result<Option<&str>, mod_api::BlockStateError> {
        use mod_api::BlockMetadata;
        mod_api::__private::block_definitions()
            .ok_or(mod_api::BlockStateError::Unavailable)?
            .property(state, name)
    }
    fn block_metadata_with(
        &self,
        state: mod_api::BlockStateRecord,
        name: &str,
        value: &str,
    ) -> Result<mod_api::BlockStateRecord, mod_api::BlockStateError> {
        use mod_api::BlockMetadata;
        mod_api::__private::block_definitions()
            .ok_or(mod_api::BlockStateError::Unavailable)?
            .with(state, name, value)
    }
    fn block_metadata_of(
        &self,
        block: mod_api::BlockKey,
        properties: &[mod_api::StateConstraint],
    ) -> Result<mod_api::BlockStateRecord, mod_api::BlockStateError> {
        use mod_api::BlockMetadata;
        mod_api::__private::block_definitions()
            .ok_or(mod_api::BlockStateError::Unavailable)?
            .of(block, properties)
    }
    fn block_state_definition(&self, block: mod_api::BlockKey) -> Option<Vec<mod_api::StateProperty>> {
        TABLES.get()?.definitions.get(self.block_name(block)?).cloned()
    }
    fn block_state_has_block_entity(&self, state: mod_api::BlockState) -> Option<bool> {
        TABLES.get()?.has_entity.get(self.block_name(state.block())?).copied()
    }
    fn block_state_correct_tool_for_drops(&self, _: mod_api::BlockState, _: &mod_api::ItemStack) -> Option<bool> {
        None
    }
    fn block_state_requires_correct_tool(&self, state: mod_api::BlockState) -> Option<bool> {
        TABLES
            .get()?
            .requires_tool
            .get(self.block_name(state.block())?)?
            .get(usize::from(state.ordinal()))
            .copied()
    }
}

impl mod_api::prelude::MenusClientHost for PojarushHost<'_> {
    fn local_player(&self) -> Option<mod_api::PlayerRef> {
        let id = self.view.local_player?;
        let uuid = self.view.entities.iter().find(|e| e.id == id)?.uuid;
        Some(mod_api::__private::game::player_ref(
            mod_api::EntityRef::new(mod_api::WorldEpoch::new(WORLD_EPOCH), id),
            uuid,
        ))
    }
}

impl mod_api::prelude::ConfigHost for PojarushHost<'_> {
    fn on(&self, mod_id: &str) -> bool {
        !self.view.off.iter().any(|id| id == mod_id)
    }
    fn setting(&self, mod_id: &str, key: &str) -> Option<String> {
        if calling_mod() != Some(mod_id) {
            return None;
        }
        mod_api::__private::setting(mod_id, key)?;
        read_setting(&self.view.settings_root.join(mod_id), key)
    }
    fn set_setting(&self, mod_id: &str, key: &str, value: &str) {
        if calling_mod() == Some(mod_id) && mod_api::__private::setting(mod_id, key).is_some() {
            store_setting(&self.view.settings_root.join(mod_id), key, value);
        }
    }
    fn config_files(&self) -> &'static dyn ConfigFiles {
        &NO_CONFIG
    }
    fn data_files(&self) -> &'static dyn ConfigFiles {
        &DATA_FILES
    }
}

impl mod_api::prelude::ConfigClientHost for PojarushHost<'_> {
    fn translate(&self, key: &str) -> Option<String> {
        self.tr(key)
    }
    fn setting_bool(&self, key: &str) -> Option<bool> {
        match self.own_setting(key)? {
            SettingValue::Toggle(on) => Some(on),
            _ => None,
        }
    }
    fn setting_choice(&self, key: &str) -> Option<u32> {
        match self.own_setting(key)? {
            SettingValue::Choice(i) => u32::try_from(i).ok(),
            _ => None,
        }
    }
    fn setting_f64(&self, key: &str) -> Option<f64> {
        match self.own_setting(key)? {
            SettingValue::Range(v) => Some(v),
            _ => None,
        }
    }
    fn setting_color(&self, key: &str) -> Option<u32> {
        match self.own_setting(key)? {
            SettingValue::Color(rgb) => Some(rgb),
            _ => None,
        }
    }
    fn official_extra_settings(&self) -> Vec<(String, String)> {
        Vec::new()
    }
}

impl mod_api::prelude::ClientInputClientHost for PojarushHost<'_> {
    fn claim_items(&mut self, _: &str, _: &[String]) {}
    fn poll(&mut self, _: &str) -> Vec<mod_api::ModEvent> {
        Vec::new()
    }
    fn cursor(&self) -> [f32; 2] {
        self.view.cursor
    }
    fn mod_key(&self, mod_id: &str, id: &str) -> Option<(String, mod_api::ClientModifiers)> {
        mod_api::__private::key_mappings()
            .iter()
            .find(|(owner, k)| *owner == mod_id && k.id == id)
            .map(|(_, k)| (k.key.clone(), mod_api::ClientModifiers::new(0)))
    }
    fn is_down(&self, _: &str, _: &str) -> bool {
        false
    }
    fn consume_click(&mut self, mod_id: &str, id: &str) -> bool {
        let mut clicks = self.clicks.borrow_mut();
        match clicks.get_mut(&(mod_id.to_owned(), id.to_owned())) {
            Some(n) if *n > 0 => {
                *n -= 1;
                true
            }
            _ => false,
        }
    }
    fn playing(&self) -> bool {
        self.view.playing
    }
    fn mapping(&self, name: &str) -> Result<Option<KeyMapping>, TextFormError> {
        Ok(self
            .view
            .mappings
            .iter()
            .find(|m| m.0 == name)
            .map(|(_, label, down)| KeyMapping {
                down: *down,
                label: Text::literal(label.clone()),
            }))
    }
    fn millis(&self) -> u64 {
        self.view.millis
    }
}

impl mod_api::prelude::ClientViewClientHost for PojarushHost<'_> {
    fn crosshair_target(&self) -> Option<mod_api::CrosshairTarget> {
        match self.view.crosshair.as_ref()? {
            Crosshair::Block { pos, block, ordinal, face, point } => {
                let key = mod_api::__private::block_key(block)?;
                Some(mod_api::CrosshairTarget::Block {
                    pos: mod_api::BlockPos::new(pos[0], pos[1], pos[2]),
                    state: mod_api::BlockStateRecord::new(key, *ordinal).into(),
                    hit: mod_api::BlockHit::new(*point, *face, false),
                })
            }
            Crosshair::Entity { id, point } => Some(mod_api::CrosshairTarget::Entity {
                entity: mod_api::EntityRef::new(mod_api::WorldEpoch::new(WORLD_EPOCH), *id),
                point: *point,
            }),
        }
    }
    fn is_paused(&self) -> bool {
        self.view.paused
    }
    fn native_corner_shown(&self, corner: Corner) -> bool {
        let i = Corner::ALL.iter().position(|c| *c == corner).unwrap_or(0);
        self.view.corners_shown.get(i).copied().unwrap_or(true)
    }
}

struct Recorder {
    cmds: Vec<DrawCmd>,
    widths: Arc<[f32]>,
    size: (f32, f32),
    /// The pose stack: translation then uniform scale.
    poses: Vec<([f32; 2], [f32; 2])>,
}

impl Recorder {
    fn new(widths: Arc<[f32]>, size: (f32, f32)) -> Self {
        Self { cmds: Vec::new(), widths, size, poses: vec![([0.0, 0.0], [1.0, 1.0])] }
    }
    fn reset_pose(&mut self) {
        self.poses.truncate(1);
    }
    fn pose(&self) -> ([f32; 2], [f32; 2]) {
        *self.poses.last().unwrap_or(&([0.0, 0.0], [1.0, 1.0]))
    }
    fn at(&self, [x, y]: [f32; 2]) -> [f32; 2] {
        let ([tx, ty], [sx, sy]) = self.pose();
        [tx + x * sx, ty + y * sy]
    }
    fn rect(&self, [x, y, w, h]: [f32; 4]) -> [f32; 4] {
        let ([_, _], [sx, sy]) = self.pose();
        let [x, y] = self.at([x, y]);
        [x, y, w * sx, h * sy]
    }
    fn push(&mut self, ops: &[mod_api::client::hud_layout::PoseOp]) {
        use mod_api::client::hud_layout::PoseOp;
        let (mut t, mut s) = self.pose();
        for op in ops {
            match *op {
                PoseOp::Translate(x, y) => {
                    t = [t[0] + x * s[0], t[1] + y * s[1]];
                }
                PoseOp::Scale(x, y) => {
                    s = [s[0] * x, s[1] * y];
                }
            }
        }
        if self.poses.len() < 64 {
            self.poses.push((t, s));
        }
    }
}

/// A text component as plain text: literals, translations through the game, nothing else.
fn plain(text: &Text, tr: &dyn Fn(&str) -> Option<String>) -> String {
    use mod_api::text::TextContent;
    let mut out = match &text.content {
        TextContent::Literal(s) => s.clone(),
        TextContent::Translatable { key, fallback, arguments } => {
            let template = tr(key).or_else(|| fallback.clone()).unwrap_or_else(|| key.clone());
            let args: Vec<String> = arguments.iter().map(|a| plain(a, tr)).collect();
            fill_template(&template, &args)
        }
        TextContent::Keybind(k) => tr(k).unwrap_or_else(|| k.clone()),
        _ => String::new(),
    };
    for child in &text.children {
        out.push_str(&plain(child, tr));
    }
    out
}

/// `%s` and `%1$s` placeholders filled in order.
fn fill_template(template: &str, args: &[String]) -> String {
    let mut out = String::new();
    let mut next = 0;
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        let mut digits = String::new();
        while let Some(d) = chars.peek().filter(|d| d.is_ascii_digit()) {
            digits.push(*d);
            chars.next();
        }
        match chars.next() {
            Some('%') => out.push('%'),
            Some('$') if !digits.is_empty() => {
                chars.next();
                let i: usize = digits.parse().unwrap_or(1);
                out.push_str(args.get(i.saturating_sub(1)).map_or("", String::as_str));
            }
            Some('s') | Some('d') => {
                out.push_str(args.get(next).map_or("", String::as_str));
                next += 1;
            }
            Some(other) => {
                out.push('%');
                out.push_str(&digits);
                out.push(other);
            }
            None => out.push('%'),
        }
    }
    out
}

impl DrawContext for Recorder {
    fn size(&self) -> [f32; 2] {
        [self.size.0, self.size.1]
    }
    fn fill(&mut self, rect: [f32; 4], argb: u32) {
        let rect = self.rect(rect);
        self.cmds.push(DrawCmd::Fill { rect, argb });
    }
    fn fill_gradient(&mut self, rect: [f32; 4], top: u32, bottom: u32) {
        let rect = self.rect(rect);
        self.cmds.push(DrawCmd::Gradient { rect, top, bottom });
    }
    fn text(&mut self, text: &str, at: [f32; 2], argb: u32, shadow: bool) {
        let scale = self.pose().1[1];
        let at = self.at(at);
        self.cmds.push(DrawCmd::Text {
            text: text.to_owned(),
            at,
            argb,
            shadow,
            scale,
        });
    }
    fn text_width(&self, text: &str) -> f32 {
        text.chars()
            .map(|c| self.widths.get(c as usize).copied().unwrap_or(6.0))
            .sum()
    }
    fn item(&mut self, item: mod_api::ItemKey, at: [f32; 2], size: f32) -> bool {
        let Some(id) = mod_api::__private::item_key_id(item) else {
            return false;
        };
        let scale = self.pose().1[1];
        let at = self.at(at);
        self.cmds.push(DrawCmd::Item { id: id.to_string(), at, size: size * scale });
        true
    }
    fn item_stack(&mut self, stack: &mod_api::ItemStack, at: [f32; 2], size: f32) -> bool {
        if stack.count() == 0 {
            return false;
        }
        self.item(stack.item(), at, size)
    }
    fn push_clip(&mut self, _rect: [f32; 4]) -> Result<(), mod_api::GuiClipRefusal> {
        Ok(())
    }
    fn pop_clip(&mut self) -> Result<(), mod_api::GuiClipRefusal> {
        Ok(())
    }
    fn gui_paint(&mut self, draws: &[mod_api::GuiDraw]) -> Result<(), TextFormError> {
        use mod_api::GuiDraw;
        let tr = |k: &str| TRANSLATE.get().and_then(|t| t(k));
        for draw in draws {
            match draw {
                GuiDraw::Fill { x0, y0, x1, y1, color } => {
                    let (a, b) = ((*x0).min(*x1) as f32, (*y0).min(*y1) as f32);
                    let (w, h) = ((x1 - x0).abs() as f32, (y1 - y0).abs() as f32);
                    self.fill([a, b, w, h], *color);
                }
                GuiDraw::FillGradient { x0, y0, x1, y1, top, bottom } => {
                    let (a, b) = ((*x0).min(*x1) as f32, (*y0).min(*y1) as f32);
                    let (w, h) = ((x1 - x0).abs() as f32, (y1 - y0).abs() as f32);
                    self.fill_gradient([a, b, w, h], *top, *bottom);
                }
                GuiDraw::Text { text, x, y, color, shadow } => {
                    text.validate_depth()?;
                    let line = plain(text, &tr);
                    DrawContext::text(self, &line, [*x as f32, *y as f32], *color, *shadow);
                }
                GuiDraw::Push(ops) => self.push(ops),
                GuiDraw::Pop => {
                    if self.poses.len() > 1 {
                        self.poses.pop();
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

/// The game's translations, for text components drawn with `gui_paint`.
static TRANSLATE: OnceLock<fn(&str) -> Option<String>> = OnceLock::new();

/// `config/`: not offered yet.
struct NoConfig;
static NO_CONFIG: NoConfig = NoConfig;
impl ConfigFiles for NoConfig {
    fn read(&self, _: &ConfigPath) -> Result<Vec<u8>, ConfigFileError> {
        Err(ConfigFileError::NotFound)
    }
    fn write(&self, _: &ConfigPath, _: &[u8]) -> Result<(), ConfigFileError> {
        Err(ConfigFileError::Io("config files are not offered in Pojarush".into()))
    }
    fn list(&self, _: &ConfigPath, _: Option<&str>, _: u32) -> Result<Vec<ConfigEntry>, ConfigFileError> {
        Ok(Vec::new())
    }
    fn stat(&self, _: &ConfigPath) -> Result<Option<ConfigStat>, ConfigFileError> {
        Ok(None)
    }
}

/// `horizon-mod-data/<calling mod>/`: each mod's bulk files.
struct DataFiles;
static DATA_FILES: DataFiles = DataFiles;

impl DataFiles {
    fn path(&self, path: &ConfigPath) -> Result<PathBuf, ConfigFileError> {
        let id = calling_mod().ok_or(ConfigFileError::NoCallingMod)?;
        let root = FILES_ROOT.get().ok_or(ConfigFileError::NoCallingMod)?;
        let mut at = root.join(id);
        for part in path.as_str().split('/').filter(|p| !p.is_empty()) {
            at.push(part);
            if std::fs::symlink_metadata(&at).is_ok_and(|m| m.file_type().is_symlink()) {
                return Err(ConfigFileError::Symlink);
            }
        }
        Ok(at)
    }
}

fn stat_of(meta: &std::fs::Metadata) -> ConfigStat {
    mod_api::__private::built::ConfigStat {
        kind: if meta.is_dir() { ConfigEntryKind::Folder } else { ConfigEntryKind::File },
        len: if meta.is_dir() { 0 } else { meta.len() },
        modified_millis: meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_millis() as u64),
    }
    .into()
}

fn io(error: std::io::Error) -> ConfigFileError {
    match error.kind() {
        std::io::ErrorKind::NotFound => ConfigFileError::NotFound,
        _ => ConfigFileError::Io(error.to_string()),
    }
}

impl ConfigFiles for DataFiles {
    fn read(&self, file: &ConfigPath) -> Result<Vec<u8>, ConfigFileError> {
        let at = self.path(file)?;
        let meta = std::fs::metadata(&at).map_err(io)?;
        if meta.is_dir() {
            return Err(ConfigFileError::NotAFile);
        }
        if meta.len() > mod_api::DATA_FILE_MAX {
            return Err(ConfigFileError::TooLarge);
        }
        std::fs::read(at).map_err(io)
    }
    fn write(&self, file: &ConfigPath, bytes: &[u8]) -> Result<(), ConfigFileError> {
        if bytes.len() as u64 > mod_api::DATA_FILE_MAX {
            return Err(ConfigFileError::TooLarge);
        }
        let at = self.path(file)?;
        if let Some(parent) = at.parent() {
            std::fs::create_dir_all(parent).map_err(io)?;
        }
        let tmp = at.with_extension("pojarush-tmp");
        std::fs::write(&tmp, bytes).map_err(io)?;
        std::fs::rename(&tmp, &at).map_err(io)
    }
    fn list(&self, folder: &ConfigPath, after: Option<&str>, limit: u32) -> Result<Vec<ConfigEntry>, ConfigFileError> {
        if limit == 0 || limit as usize > mod_api::CONFIG_LIST_MAX {
            return Err(ConfigFileError::InvalidPage);
        }
        let at = self.path(folder)?;
        let mut names: Vec<(String, std::fs::Metadata)> = match std::fs::read_dir(&at) {
            Ok(dir) => dir
                .flatten()
                .filter_map(|e| Some((e.file_name().into_string().ok()?, e.metadata().ok()?)))
                .collect(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(io(e)),
        };
        names.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(names
            .into_iter()
            .filter(|(n, _)| after.is_none_or(|a| n.as_str() > a))
            .take(limit as usize)
            .map(|(name, meta)| {
                mod_api::__private::built::ConfigEntry { name, stat: stat_of(&meta) }.into()
            })
            .collect())
    }
    fn stat(&self, path: &ConfigPath) -> Result<Option<ConfigStat>, ConfigFileError> {
        let at = self.path(path)?;
        match std::fs::metadata(&at) {
            Ok(meta) => Ok(Some(stat_of(&meta))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(io(e)),
        }
    }
}
