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
use std::collections::HashMap;
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
}

/// What a `render_gui_post` hook drew, in GUI pixels, for the game to draw in order.
#[derive(Clone, Debug, PartialEq)]
pub enum DrawCmd {
    Fill { rect: [f32; 4], argb: u32 },
    Text { text: String, at: [f32; 2], argb: u32, shadow: bool },
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
    /// `game_dir/.horizon-wasm-cache`, so later starts are quick). Hooks do nothing until it is
    /// done. Once per process.
    pub fn start(mods_dir: &Path, game_dir: &Path, off: &[String]) -> Self {
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
        let (shared2, ready2) = (shared.clone(), ready.clone());
        let spawned = horizon_guest_stack::builder()
            .name("wasm-mods".into())
            .spawn(move || {
                let mut inner = Inner::load(entries, &game_dir);
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
    /// Translations from the packages' `assets/<ns>/lang/<code>.json`, by language.
    lang: HashMap<String, HashMap<String, String>>,
}

static FILES_ROOT: OnceLock<PathBuf> = OnceLock::new();

impl Inner {
    fn load(mut entries: Vec<Entry>, game_dir: &Path) -> Self {
        let _ = FILES_ROOT.set(game_dir.join("horizon-mod-data"));
        let mut lang: HashMap<String, HashMap<String, String>> = HashMap::new();
        let mut packages = Vec::new();
        if let Err(why) = horizon_guest_stack::check_current() {
            for e in entries.iter_mut().filter(|e| e.problem.as_deref() == Some(STARTING)) {
                e.problem = Some(why.to_owned());
            }
            return Self { entries, running: false, lang };
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
        if !packages.is_empty() {
            let started = std::time::Instant::now();
            let loaded = horizon_mod_wasm::load_cached(
                STD_MEMBER,
                packages,
                &game_dir.join(".horizon-wasm-cache"),
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
                running = start_phases(&mut entries);
            }
            log::info!(
                "wasm mods: {} running, {} refused, in {:?}",
                entries.iter().filter(|e| e.problem.is_none()).count(),
                refused.len(),
                started.elapsed()
            );
        }
        Self { entries, running, lang }
    }

    fn host<'a>(&'a self, view: &'a View) -> PojarushHost<'a> {
        PojarushHost { view, lang: &self.lang }
    }

    fn render_gui_post(&self, view: &View, size: (f32, f32), used: [f32; 4]) -> (Vec<DrawCmd>, [f32; 4]) {
        let mut corners = Corners::from_parts(size.0, size.1, used);
        let mut out = Recorder { cmds: Vec::new(), widths: view.widths.clone() };
        let host = self.host(view);
        guarded("render_gui_post", || {
            mod_api::dispatch_render_gui_post(&host, &mut out, &mut corners)
        });
        (out.cmds, corners.parts().2)
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
fn start_phases(entries: &mut [Entry]) -> bool {
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

impl Host for PojarushHost<'_> {}
mod_api::impl_host_areas!(PojarushHost<'_>, except(config));
impl ClientHost for PojarushHost<'_> {}
mod_api::impl_client_host_areas!(PojarushHost<'_>, except(client_input, client_view, config));

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
    fn mod_key(&self, _: &str, _: &str) -> Option<(String, mod_api::ClientModifiers)> {
        None
    }
    fn is_down(&self, _: &str, _: &str) -> bool {
        false
    }
    fn consume_click(&mut self, _: &str, _: &str) -> bool {
        false
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
    fn native_corner_shown(&self, corner: Corner) -> bool {
        let i = Corner::ALL.iter().position(|c| *c == corner).unwrap_or(0);
        self.view.corners_shown.get(i).copied().unwrap_or(true)
    }
}

struct Recorder {
    cmds: Vec<DrawCmd>,
    widths: Arc<[f32]>,
}

impl DrawContext for Recorder {
    fn fill(&mut self, rect: [f32; 4], argb: u32) {
        self.cmds.push(DrawCmd::Fill { rect, argb });
    }
    fn text(&mut self, text: &str, at: [f32; 2], argb: u32, shadow: bool) {
        self.cmds.push(DrawCmd::Text {
            text: text.to_owned(),
            at,
            argb,
            shadow,
        });
    }
    fn text_width(&self, text: &str) -> f32 {
        text.chars()
            .map(|c| self.widths.get(c as usize).copied().unwrap_or(6.0))
            .sum()
    }
}

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
