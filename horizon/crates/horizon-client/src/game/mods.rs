//! Horizon mods in the game (`docs/mods.md`): loading the ones turned on,
//! their tick, frame, chat and command hooks, their HUD panels and what
//! they print. With no mod on, `horizon_mods::Mods` holds no engine and
//! every hook here returns after one check.
//!
//! Code mods run only in builds with the `mods` feature (release builds):
//! without it the game links no wasmtime, and every hook does nothing.
//! Content mods (`content_root`) load in every build.
use super::Game;
use horizon_client::{
    command::{Dispatcher, vanilla},
    extras_hud::Corners,
    hud::HudBatch,
    mod_list,
};
use std::{path::PathBuf, time::Duration};
#[cfg(feature = "mods")]
use {
    super::Screen,
    super::{execution_role::ExecutionRole, lan::LanHost, server_mods::ModEvent},
    horizon_client::{
        extras::Corner,
        extras_hud,
        items::ItemStack,
        keybinds,
        mod_list::Installed,
        options_ui::Page,
        text::{self, Text},
        ui,
    },
    horizon_mods::{
        Clock, ModKey, Mods, Output, Player, Runs, ServerPlayer, Snapshot, Status, UiScreen,
        UiWidget,
    },
    std::time::Instant,
    winit::{event::MouseButton, keyboard::KeyCode},
};

#[derive(Default)]
pub(super) struct ModsState {
    #[cfg(feature = "mods")]
    pub(super) mods: Mods,
    /// Capability `input`: a claimed item's use while the button is down.
    #[cfg(feature = "mods")]
    pub(super) using: Option<super::mod_input::ModUse>,
    /// Capability `render`'s camera shakes.
    #[cfg(feature = "mods")]
    pub(super) shake: super::mod_render::Shake,
    /// `--dev-mod PATH`: always on, reloaded when rebuilt.
    pub(super) dev: Vec<PathBuf>,
    /// The options page a mod's settings page was opened from, to go back to.
    pub(super) settings_from: Option<horizon_client::options_ui::Page>,
    /// A mod's draggable HUD area held with the mouse: the mod and the
    /// cursor's last position in GUI pixels.
    #[cfg(feature = "mods")]
    pub(super) hud_mouse: Option<(usize, f32, f32)>,
    /// When dev mods were last checked for a rebuild.
    #[cfg(feature = "mods")]
    checked: Option<Instant>,
    /// The chat's command tree while a running mod owns commands.
    #[cfg(feature = "mods")]
    tree: Option<Dispatcher>,
    /// A server mod runs here: what players do is queued for it.
    #[cfg(feature = "mods")]
    pub(super) server: bool,
    /// The players server mods saw last tick, for joins and leaves.
    #[cfg(feature = "mods")]
    pub(super) players: Vec<ServerPlayer>,
    /// What players did this tick, for server mods at its end.
    #[cfg(feature = "mods")]
    pub(super) events: Vec<ModEvent>,
    /// A running server mod has capability `world`: its calls get the
    /// levels' chunks to read.
    #[cfg(feature = "mods")]
    pub(super) world: bool,
    /// Blocks server mods set, by dimension, made before that level's next
    /// tick.
    #[cfg(feature = "mods")]
    pub(super) block_writes: Vec<(String, horizon_mods::BlockChange)>,
    /// `command.run` lines for this window's own world, run on the next tick:
    /// run at once, their chat lines would reach the mods while their output
    /// is still being shown, and a mod answering its own lines would recurse.
    #[cfg(feature = "mods")]
    commands: Vec<String>,
    /// `command.run-quiet` batches for this window's own world, by mod id,
    /// run on the next tick as `commands` are.
    #[cfg(feature = "mods")]
    quiet: Vec<(String, Vec<String>)>,
    /// The server's command tree with the running mods' commands added.
    #[cfg(feature = "mods")]
    remote_tree: Option<Dispatcher>,
    /// A `ui` mod's screen or settings page while it is up.
    #[cfg(feature = "mods")]
    screen: Option<ModScreen>,
    /// The mod whose title-screen button was just clicked: its screen may open over the title.
    #[cfg(feature = "mods")]
    title_click: Option<usize>,
    /// The HUD font's advances for `render.text-width`, made once.
    #[cfg(feature = "mods")]
    font_widths: std::sync::Arc<[f32]>,
    /// Capability `recipes`'s tables, made once a mod reads them.
    #[cfg(feature = "mods")]
    pub(super) recipes: Option<std::sync::Arc<horizon_mods::Recipes>>,
    /// Extras that are official mods now, being installed.
    #[cfg(feature = "mods")]
    pub(super) moved: super::moved_extras::MovedState,
    /// Playing at the last frame, for `mod_input_frame`.
    #[cfg(feature = "mods")]
    pub(super) playing: bool,
}

/// The mappings `input.mapping` answers for, with the default key their
/// binding is translated to (none: a mouse button, attack or use).
#[cfg(feature = "mods")]
const MAPPINGS: [(&str, Option<winit::keyboard::KeyCode>); 9] = {
    use winit::keyboard::KeyCode as K;
    [
        ("key.forward", Some(K::KeyW)),
        ("key.left", Some(K::KeyA)),
        ("key.back", Some(K::KeyS)),
        ("key.right", Some(K::KeyD)),
        ("key.jump", Some(K::Space)),
        ("key.sneak", Some(K::ShiftLeft)),
        ("key.sprint", Some(K::ControlLeft)),
        ("key.attack", None),
        ("key.use", None),
    ]
};
/// Characters `render.text-width` knows exactly: U+0000 to U+024F.
#[cfg(feature = "mods")]
const FONT_CHARS: u32 = 0x250;

/// A `ui` mod's own screen, or its settings page from Options > Mods: the
/// widgets the mod pushed, laid out and drawn with the menus' own widgets.
#[cfg(feature = "mods")]
struct ModScreen {
    /// The running mod's index in `mods.mods.entries()`.
    owner: usize,
    /// Opened from Options > Mods: Done and Esc return there.
    settings: bool,
    /// The mod's own screen shown for its settings (`settings-opened`
    /// opened it): Done and Esc return to Options too.
    back: bool,
    /// What the mod's screen replaced (Playing, Chat, ...), to restore.
    under: Screen,
    /// One scroll per `list` widget, in widget order.
    scrolls: Vec<ui::Scroll>,
    /// A key row waiting for the key to bind: the mod key's name.
    binding: Option<String>,
    /// The slider widget being dragged: its id.
    slider: Option<u32>,
    /// How far the page is scrolled down, in GUI pixels.
    page: f32,
    /// A drag on empty space scrolling the page: where it started and the scroll then.
    pan: Option<(f32, f32)>,
}
impl ModsState {
    /// The chat's command tree: vanilla's, plus the running mods' commands.
    pub(super) fn dispatcher(&self) -> &Dispatcher {
        #[cfg(feature = "mods")]
        if let Some(tree) = &self.tree {
            return tree;
        }
        vanilla::game_dispatcher()
    }
    /// The server's tree plus the mods' commands, when a mod owns any.
    pub(super) fn remote_dispatcher(&self) -> Option<&Dispatcher> {
        #[cfg(feature = "mods")]
        if let Some(tree) = &self.remote_tree {
            return Some(tree);
        }
        None
    }
    /// Whether a mod runs: the chat keeps its lines for them.
    pub(super) fn running(&self) -> bool {
        #[cfg(feature = "mods")]
        if self.mods.is_running() {
            return true;
        }
        false
    }
    /// Whether a mod draws a HUD panel or a corner panel.
    pub(super) fn has_panels(&self) -> bool {
        #[cfg(feature = "mods")]
        if self.mods.panels().next().is_some() || self.mods.corner_panels().next().is_some() {
            return true;
        }
        false
    }
}
/// How often dev mods are checked for a rebuild.
#[cfg(feature = "mods")]
const RELOAD_CHECK: Duration = Duration::from_secs(1);

/// A vanilla command name: mods may not take it.
#[cfg(feature = "mods")]
fn vanilla_command(name: &str) -> bool {
    let d = vanilla::game_dispatcher();
    d.nodes[0]
        .children
        .iter()
        .any(|&c| d.nodes[c].name() == name)
}

/// `root` with the content mods that are on layered over it
/// (docs/modpacks.md): their blocks, items, data and assets. Read once at
/// start, before the resources load; a mod whose content is refused is left
/// out here and its row on the Mods page says why.
pub(super) fn content_root(
    root: &std::path::Path,
    enabled: &std::collections::BTreeSet<String>,
    dev: &[std::path::PathBuf],
) -> std::path::PathBuf {
    use horizon_client::mod_content::{self, ContentMod};
    use horizon_mods::{Capability, Content, Package};
    let here = std::env::current_dir().unwrap_or_default();
    let mut paths = horizon_mods::find(&here.join(mod_list::DIR));
    paths.retain(|p| !dev.contains(p));
    paths.extend(dev.iter().cloned());
    let mut mods: Vec<ContentMod> = Vec::new();
    for path in &paths {
        let Ok(package) = Package::read(path) else {
            continue;
        };
        let m = &package.manifest;
        let on = enabled.contains(&m.id) || dev.contains(path);
        if !on
            || !m.capabilities.contains(&Capability::Registries)
            || mods.iter().any(|c| c.id == m.id)
        {
            continue;
        }
        let content = Content::read(path, m).and_then(|content| {
            // The first of two mods owning one namespace stays on.
            let own = (m.id.as_str(), content.namespaces.as_slice());
            match mods
                .iter()
                .find_map(|c| horizon_mods::content::clash((&c.id, &c.content.namespaces), own))
            {
                Some(e) => Err(e),
                None => Ok(content),
            }
        });
        match content {
            Ok(content) => mods.push(ContentMod {
                id: m.id.clone(),
                content,
            }),
            Err(e) => log::warn!("Horizon: mod {} content not loaded: {e}", m.id),
        }
    }
    match mod_content::resource_root(root, &mods, &super::packs::cache_root()) {
        Ok(out) => {
            if out != root {
                log::info!(
                    "Horizon: content from {} mod(s) in {}",
                    mods.len(),
                    out.display()
                );
            }
            out
        }
        Err(e) => {
            log::warn!("Horizon: content mods not loaded: {e}");
            root.to_owned()
        }
    }
}

/// A build without the `mods` feature: no mod runs.
#[cfg(not(feature = "mods"))]
impl Game<'_> {
    /// Says why the installed and dev mods don't run.
    pub(super) fn start_mods(&mut self) {
        let dir = Self::options_path().with_file_name(mod_list::DIR);
        let found = horizon_mods::find(&dir).len() + self.mods.dev.len();
        if found > 0 {
            log::warn!(
                "Horizon: {found} mod(s) not started: this build has no mod runtime (build with --features mods)"
            );
        }
        self.list_native_mods();
    }
    pub(super) fn merge_remote_tree(&mut self) {}
    pub(super) fn sync_mods(&mut self) {}
    pub(super) fn tick_mods(&mut self) {}
    pub(super) fn mods_frame(&mut self, _elapsed: Duration) {}
    pub(super) fn mod_command(&mut self, _command: &str) -> bool {
        false
    }
    pub(super) fn mods_chat(&mut self, _line: &str) {}
    pub(super) fn draw_mod_panels(&self, _batch: &mut HudBatch, _corners: &mut Corners) {}
    pub(super) fn open_mod_screen(&mut self, _owner: usize, _settings: bool) {}
    pub(super) fn mod_title_button(&mut self) {}
    pub(super) fn mod_button_icon(&mut self) -> Option<[f32; 4]> {
        None
    }
    pub(super) fn mods_title_button(&self) -> Option<(usize, String)> {
        None
    }
    pub(super) fn mod_video_button(&mut self) {}
    pub(super) fn close_mod_screen(&mut self, _tell: bool) {}
    pub(super) fn draw_mod_screen(&mut self, _batch: &mut HudBatch) {}
    pub(super) fn mod_screen_click(&mut self, _button: winit::event::MouseButton) {}
    pub(super) fn mod_screen_key(&mut self, _key: winit::keyboard::KeyCode) -> bool {
        false
    }
    pub(super) fn mod_screen_scroll(&mut self, _delta: f32) {}
    pub(super) fn mod_screen_drag(&mut self) {}
    pub(super) fn mod_screen_release(&mut self) {}
    pub(super) fn mods_hide_hitboxes(&self) -> bool {
        false
    }
    pub(super) fn mod_world_lines(&self) -> (Vec<horizon_client::gizmos::Line>, Vec<horizon_client::gizmos::Line>) {
        (Vec::new(), Vec::new())
    }
}
/// What the launcher writes beside a game it built with native mods: their ids under `mods`.
const BUILD_INFO: &str = "native-mods.json";

#[cfg(feature = "mods")]
impl Game<'_> {
    /// Reads `horizon-mods/` and the dev mods, and starts the ones turned on.
    pub(super) fn start_mods(&mut self) {
        let dir = Self::options_path().with_file_name(mod_list::DIR);
        let dev = &self.mods.dev;
        let mut paths = horizon_mods::find(&dir);
        paths.retain(|p| !dev.contains(p));
        paths.extend(dev.iter().cloned());
        // A dev mod is on whatever the list says.
        let dev_ids: Vec<String> = dev
            .iter()
            .filter_map(|p| horizon_mods::Package::read(p).ok())
            .map(|p| p.manifest.id)
            .collect();
        // A native mod of the same id (one built into this game) takes over.
        let mut native = Vec::new();
        self.native_mods(&mut native);
        let enabled = &self.settings.mods.enabled;
        let on = |id: &str| {
            (enabled.contains(id) || dev_ids.iter().any(|d| d == id))
                && !native.iter().any(|n| n.id == id)
        };
        let mods = Mods::load_for(self.mod_runs(), &paths, on, vanilla_command);
        self.mods.mods = mods;
        let chosen = &self.settings.mods;
        self.mods.mods.set_bindings(|id| chosen.keys_of(id));
        // Entries were replaced wholesale: a screen's owner index is stale.
        self.mods.screen = None;
        // Content mods may have changed the items: remade on the next read.
        self.mods.recipes = None;
        self.list_mods();
        self.sync_mod_commands();
        self.show_mod_output();
    }
    /// Which mods this game runs: a server the server mods, the window
    /// client mods, and both when it simulates the world itself (in-frame).
    fn mod_runs(&self) -> Runs {
        match self.options.execution_role {
            ExecutionRole::IntegratedServer | ExecutionRole::DedicatedServer => Runs::Server,
            ExecutionRole::Client if self.options.server_thread => Runs::Client,
            ExecutionRole::Client => Runs::Both,
        }
    }
    /// The Mods page's rows, from what loaded.
    fn list_mods(&mut self) {
        self.mods.server = self.mods.mods.runs_server();
        self.mods.world = self.mods.mods.wants_world();
        let dev = &self.mods.dev;
        self.settings.mods.installed = self
            .mods
            .mods
            .entries()
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let keys: Vec<[String; 3]> = self
                    .mods
                    .mods
                    .keys_of(i)
                    .iter()
                    .map(|k| [k.name.clone(), k.label.clone(), k.default.clone()])
                    .collect();
                let file = e.path.file_name().unwrap_or_default().to_string_lossy();
                let problem = match &e.status {
                    Status::Failed(reason) => Some(format!("Stopped: {reason}")),
                    _ => None,
                };
                let Some(m) = &e.manifest else {
                    return Installed {
                        id: String::new(),
                        title: file.into_owned(),
                        description: String::new(),
                        problem,
                        dev: false,
                        has_settings: false,
                        native: None,
                        details: Default::default(),
                    };
                };
                // What it may do first: the row cuts long descriptions.
                let may: Vec<&str> = m.capabilities.iter().map(|c| c.describe()).collect();
                let may = if may.is_empty() {
                    "Asks for nothing.".to_owned()
                } else {
                    format!("May {}.", may.join(", "))
                };
                let (name, description) = m.localized(&horizon_client::lang::code());
                Installed {
                    id: m.id.clone(),
                    title: format!("{name} {}", m.version),
                    description: format!("{may} {description}").trim().to_owned(),
                    problem,
                    dev: dev.contains(&e.path),
                    has_settings: e.has_ui_settings(),
                    native: None,
                    details: mod_list::ModDetails {
                        name,
                        version: m.version.clone(),
                        description,
                        authors: m.authors.clone(),
                        license: m.license.clone(),
                        minecraft: m.minecraft.clone(),
                        homepage: m.contact.homepage.clone(),
                        issues: m.contact.issues.clone(),
                        sources: m.contact.sources.clone(),
                        icon: None,
                        keys,
                    },
                }
            })
            .collect();
        self.list_native_mods();
    }
    /// The chat's tree after the running mods changed.
    fn sync_mod_commands(&mut self) {
        // In-frame, the window runs server mods too: their names, free text
        // here (the server parses their arguments).
        let server = self.mods.mods.server_commands().map(|c| c.name.as_str());
        let mut names = self.mods.mods.commands().chain(server).peekable();
        self.mods.tree = names
            .peek()
            .is_some()
            .then(|| vanilla::game_dispatcher().with_mod_commands(names));
        self.merge_remote_tree();
    }

    /// The server's command tree with the mods' commands, for the chat.
    pub(super) fn merge_remote_tree(&mut self) {
        let names: Vec<String> = self.mods.mods.commands().map(str::to_owned).collect();
        self.mods.remote_tree = self
            .remote
            .as_ref()
            .and_then(|r| r.commands.as_ref())
            .filter(|_| !names.is_empty())
            .map(|t| t.with_mod_commands(names.iter().map(String::as_str)));
    }

    /// After the options changed: restart the mods if the list of the ones
    /// turned on did.
    pub(super) fn sync_mods(&mut self) {
        let chosen = &self.settings.mods;
        self.mods.mods.set_bindings(|id| chosen.keys_of(id));
        let running: Vec<&str> = self
            .mods
            .mods
            .entries()
            .iter()
            .filter(|e| e.status != Status::Off)
            .filter_map(|e| Some(e.manifest.as_ref()?.id.as_str()))
            .collect();
        let wanted: Vec<&str> = self
            .settings
            .mods
            .installed
            .iter()
            .enumerate()
            .filter(|&(i, _)| self.settings.mods.is_on(i))
            .map(|(_, m)| m.id.as_str())
            .collect();
        if running != wanted {
            self.start_mods();
        }
    }
    /// What mods read this tick (or frame, for mods with a `frame` hook): the
    /// server's players as they were at the last tick's end.
    fn mod_snapshot(&mut self) -> Snapshot {
        let players = if self.mods.server {
            self.mods.players.clone()
        } else {
            Vec::new()
        };
        self.mod_snapshot_with(players)
    }
    /// What mods read, with these players on the server.
    pub(super) fn mod_snapshot_with(&mut self, players: Vec<ServerPlayer>) -> Snapshot {
        let player = self.player.as_ref().map(|p| Player {
            x: p.position.x,
            y: p.position.y,
            z: p.position.z,
            yaw: self.yaw,
            pitch: self.pitch,
            health: p.health.health(),
            food: p.health.food.food.max(0) as u32,
            dimension: self.dim.id().to_owned(),
        });
        let clock = player.is_some().then(|| Clock {
            game_time: self.world.game_time.max(0) as u64,
            day_time: self.world.day_time.max(0) as u64,
        });
        let (cps, mouse) = self.mouse_reading();
        // The movement and click mappings, as the keystroke overlay reads them:
        // keys by the default key their binding is translated to.
        let mappings = MAPPINGS
            .iter()
            .map(|&(name, key)| {
                let label = keybinds::display_name(self.settings.keys.get(name)).plain();
                let down = match key {
                    Some(k) => self.keys.contains(&k),
                    None => mouse[usize::from(name == "key.use")],
                };
                (name, label, down)
            })
            .collect();
        if self.mods.font_widths.is_empty() {
            let atlas = &self.gui.atlas;
            self.mods.font_widths = (0..FONT_CHARS)
                .map(|c| char::from_u32(c).map_or(6.0, |c| atlas.glyph_cell(c).1))
                .collect();
        }
        let (recipes, known_recipes) = self.mod_recipes();
        Snapshot {
            player,
            clock,
            gui_size: self.gui_size(),
            held: self.mod_held(),
            frame: self.frame_reading(),
            mappings,
            cps,
            font_widths: self.mods.font_widths.clone(),
            players,
            view_hit: self.mod_view_hit(),
            entities: self.mod_entities(),
            hitboxes_shown: self.show_hitboxes,
            recipes,
            known_recipes,
            placing: self.mod_recipe_screen(),
            creative: self.creative,
        }
    }
    /// The entities around the player as this frame draws them
    /// (`game.entities`), from the hitboxes F3+B shows.
    fn mod_entities(&self) -> std::sync::Arc<[horizon_mods::ModEntity]> {
        if !self.mods.mods.wants_entities() {
            return std::sync::Arc::from([]);
        }
        let Some(p) = &self.player else {
            return std::sync::Arc::from([]);
        };
        let alpha = self.clock.fraction() as f32;
        let mut g = horizon_client::gizmos::Gizmos {
            record: true,
            ..Default::default()
        };
        self.hitbox_gizmos(alpha, &mut g);
        let target = self
            .mob_hit()
            .filter(|&(_, d)| self.before_block(d))
            .map(|(i, _)| self.mobs[i].id);
        let at = [p.position.x, p.position.y, p.position.z];
        let max = f64::from(horizon_mods::ENTITIES_MAX);
        let mut out: Vec<(f64, horizon_mods::ModEntity)> = g
            .boxes
            .into_iter()
            .filter_map(|b| {
                let d2 = (0..3).map(|i| (b.position[i] - at[i]).powi(2)).sum::<f64>();
                (d2 <= max * max).then(|| {
                    let targeted = !b.tag.own && b.tag.id != 0 && Some(b.tag.id) == target;
                    (
                        d2,
                        horizon_mods::ModEntity {
                            id: b.tag.id,
                            kind: b.tag.kind,
                            x: b.position[0],
                            y: b.position[1],
                            z: b.position[2],
                            min_x: b.min[0],
                            min_y: b.min[1],
                            min_z: b.min[2],
                            max_x: b.max[0],
                            max_y: b.max[1],
                            max_z: b.max[2],
                            eye_height: b.eye_height,
                            living: b.living,
                            yaw: b.view.1,
                            pitch: b.view.0,
                            hurt_time: b.tag.hurt_time,
                            vehicle: b.tag.vehicle,
                            targeted,
                            mine: b.tag.own,
                        },
                    )
                })
            })
            .collect();
        out.sort_by(|a, b| a.0.total_cmp(&b.0));
        out.into_iter().map(|(_, e)| e).collect()
    }
    /// A running mod draws the hitboxes itself (`render.hide-hitboxes`).
    pub(super) fn mods_hide_hitboxes(&self) -> bool {
        self.mods.mods.hides_hitboxes()
    }
    /// The running mods' world shapes as gizmo lines: (depth-tested, on top).
    pub(super) fn mod_world_lines(&self) -> (Vec<horizon_client::gizmos::Line>, Vec<horizon_client::gizmos::Line>) {
        use horizon_mods::WorldShape as S;
        let mut lines = horizon_client::gizmos::Gizmos::default();
        let mut top = horizon_client::gizmos::Gizmos::default();
        let p = |w: &horizon_mods::WorldPos| [w.x, w.y, w.z];
        for (shapes, on_top) in self.mods.mods.world_shapes() {
            for (list, g) in [(shapes, &mut lines), (on_top, &mut top)] {
                for shape in list {
                    match shape {
                        S::Line(l) => g.line(p(&l.start), p(&l.end), l.color, l.width),
                        S::Arrow(l) => g.arrow(p(&l.start), p(&l.end), l.color, l.width),
                        S::Cuboid(b) => g.cuboid(p(&b.min), p(&b.max), b.color, b.width),
                        S::Point(pt) => g.point(p(&pt.at), pt.color, pt.size),
                    }
                }
            }
        }
        (lines.lines, top.lines)
    }
    /// The first block along the player's view within
    /// [`horizon_mods::RAYCAST_MAX`] and its distance from the eye
    /// (`game.raycast`): through the loaded chunks as the crosshair's ray
    /// goes (fluids pass), then across the distant terrain's ground heights,
    /// a block at a time.
    fn mod_view_hit(&self) -> Option<(f32, horizon_mods::Hit)> {
        if !self.mods.mods.wants_raycast() {
            return None;
        }
        let (eye, dir) = self.aim()?;
        let reach = f64::from(horizon_mods::RAYCAST_MAX);
        let (pos, point, state) = match self.clip_ray(eye, dir, reach) {
            Some(hit) => (
                hit.pos,
                hit.point,
                self.world.block(hit.pos[0], hit.pos[1], hit.pos[2]),
            ),
            None => self.lod_ray(eye, dir, reach)?,
        };
        let distance = (0..3).map(|i| (point[i] - eye[i]).powi(2)).sum::<f64>();
        let hit = horizon_mods::Hit {
            block_x: pos[0],
            block_y: pos[1],
            block_z: pos[2],
            x: point[0],
            y: point[1],
            z: point[2],
            block: self.res.facts().get(state).name.to_string(),
        };
        Some((distance.sqrt() as f32, hit))
    }
    /// Past the resident chunks: the view walked across the distant
    /// terrain's ground heights (`Lod::ground`), a block at a time. Columns in
    /// loaded chunks are left to `clip_ray`, which has already missed them.
    fn lod_ray(
        &self,
        eye: [f64; 3],
        dir: [f64; 3],
        reach: f64,
    ) -> Option<([i32; 3], [f64; 3], horizon_world::registry::BlockStateId)> {
        let lod = self.lod.as_ref()?;
        let mut t = 0.0;
        while t <= reach {
            let p = [0, 1, 2].map(|i| eye[i] + dir[i] * t);
            let (x, z) = (p[0].floor() as i32, p[2].floor() as i32);
            if !self.world.loaded(x, z)
                && let Some((height, state)) = lod.ground(x, z)
                && p[1] <= f64::from(height)
            {
                return Some(([x, height - 1, z], [p[0], f64::from(height), p[2]], state));
            }
            t += 1.0;
        }
        None
    }
    /// A mod's `command.run-quiet` lines (vanilla's commands only, as
    /// `command.run`): to a server that runs them quietly as one payload
    /// (`horizon_protocol::quiet`), to any other as chat commands, or for this
    /// window's own world next tick.
    fn mod_quiet_commands(&mut self, mod_id: String, lines: Vec<String>) {
        let lines: Vec<String> = lines
            .into_iter()
            .filter(|l| vanilla_command(l.split(' ').next().unwrap_or_default()))
            .collect();
        match self.remote.as_ref().map(|r| r.quiet_commands) {
            Some(true) => {
                for data in horizon_protocol::quiet::encode(&mod_id, &lines) {
                    self.send(horizon_protocol::packets::play::Serverbound::CustomPayload(
                        horizon_protocol::packets::configuration::CustomPayload {
                            channel: horizon_protocol::quiet::CHANNEL.into(),
                            data,
                        },
                    ));
                }
            }
            // The server checks the player's permissions; its feedback shows.
            Some(false) => {
                for line in lines {
                    self.send_chat(&format!("/{line}"));
                }
            }
            None => self.mods.quiet.push((mod_id, lines)),
        }
    }
    /// A quiet batch in this window's own world, as a server runs one
    /// (game/lan_quiet.rs): each line as the player typing it, no chat, a
    /// chat message refused; one line in the log.
    fn run_quiet_batch(&mut self, mod_id: &str, lines: &[String]) {
        let permission = self.ui.registries.permission;
        let mut failed = 0;
        for line in lines {
            let ok = self.signable_arguments(permission, line).is_empty()
                && self.run_command_quietly(line);
            failed += usize::from(!ok);
        }
        log::info!(
            "[mod {mod_id}] ran {} quiet command(s): {failed} failed",
            lines.len()
        );
    }
    /// A game tick.
    pub(super) fn tick_mods(&mut self) {
        // Here, at the player's level: shown inside a console call (a control
        // socket command set the hook off), they would run as the console.
        for line in std::mem::take(&mut self.mods.commands) {
            self.run_command(&line);
        }
        for (mod_id, lines) in std::mem::take(&mut self.mods.quiet) {
            self.run_quiet_batch(&mod_id, &lines);
        }
        if !self.mods.mods.is_running() {
            return;
        }
        self.tick_sandbox_mod_use();
        let snapshot = self.mod_snapshot();
        self.mods.mods.set_snapshot(snapshot);
        self.mods.mods.tick(self.ticks);
        self.show_mod_output();
    }
    /// Restarts the dev mods rebuilt since the last check, once a second.
    pub(super) fn reload_dev_mods(&mut self) {
        if self.mods.dev.is_empty()
            || self
                .mods
                .checked
                .is_some_and(|t| t.elapsed() < RELOAD_CHECK)
        {
            return;
        }
        self.mods.checked = Some(Instant::now());
        if self.mods.mods.reload_changed(vanilla_command) {
            let chosen = &self.settings.mods;
            self.mods.mods.set_bindings(|id| chosen.keys_of(id));
            self.list_mods();
            self.sync_mod_commands();
            if self.options.execution_role.is_client() {
                self.system_message(Text::literal("Mods reloaded").with_color(text::GRAY));
            } else {
                log::info!("Horizon: mods reloaded");
            }
        }
    }
    /// Once per frame: the frame hook, and dev mods' rebuilds.
    pub(super) fn mods_frame(&mut self, elapsed: Duration) {
        self.reload_dev_mods();
        if !self.mods.mods.is_running() {
            return;
        }
        self.mod_input_frame();
        if self.mods.mods.wants_frames() {
            let snapshot = self.mod_snapshot();
            self.mods.mods.set_snapshot(snapshot);
        }
        self.mods.mods.frame(elapsed.as_secs_f32());
        self.show_mod_output();
        self.mod_ui_requests();
    }
    /// Shares the mod list and the shader packs with `ui.mods` and
    /// `ui.shader-*`, then does what mods asked of the menus.
    fn mod_ui_requests(&mut self) {
        static SCANNED: std::sync::Mutex<Option<Instant>> = std::sync::Mutex::new(None);
        let due = SCANNED.lock().is_ok_and(|mut last| {
            let due = last.is_none_or(|t| t.elapsed() >= Duration::from_secs(2));
            if due {
                *last = Some(Instant::now());
            }
            due
        });
        let button = self.mods.mods.title_button();
        let icon = self.mod_button_icon();
        if let Some(title) = self.title.as_mut() {
            let label = button.map(|(_, l)| l);
            if title.mod_button != label {
                title.mod_button = label;
            }
            title.mod_icon = icon;
        }
        let video = self.mods.mods.video_button().map(|(_, l)| l);
        if self.ui.options.video_button != video {
            self.ui.options.video_button = video;
        }
        if self.ui.options.page() == horizon_client::options_ui::Page::ModMenu {
            let icons: Vec<(PathBuf, String)> = self
                .mods
                .mods
                .entries()
                .iter()
                .map(|e| (e.path.clone(), e.manifest.as_ref().map(|m| m.icon.clone()).unwrap_or_default()))
                .collect();
            for (i, (path, icon)) in icons.iter().enumerate() {
                let rect = self.mod_icon(i, path, icon);
                if let Some(row) = self.settings.mods.installed.get_mut(i) {
                    row.details.icon = rect;
                }
            }
        }
        if due {
            self.mods.mods.publish_directory();
            let packs = horizon_client::shader_pack_list::scan(&Self::shader_packs_dir())
                .into_iter()
                .map(|p| p.file)
                .collect();
            horizon_mods::set_shader_directory(packs, self.settings.horizon.shader_pack.clone());
        }
        for r in horizon_mods::take_ui_requests() {
            match r {
                horizon_mods::UiRequest::ModSettings(id) => {
                    let found = self.mods.mods.entries().iter().position(|e| {
                        e.manifest.as_ref().is_some_and(|m| m.id == id)
                    });
                    if let Some(i) = found {
                        self.mods.screen = None;
                        self.open_mod_screen(i, true);
                    }
                }
                horizon_mods::UiRequest::ShaderPack(name) => {
                    if self.settings.horizon.shader_pack != name {
                        self.settings.horizon.shader_pack = name.clone();
                        self.settings.save(&Self::options_path());
                        horizon_mods::set_shader_directory(
                            horizon_client::shader_pack_list::scan(&Self::shader_packs_dir())
                                .into_iter()
                                .map(|p| p.file)
                                .collect(),
                            name,
                        );
                    }
                }
                horizon_mods::UiRequest::ShaderOptions => {
                    if !self.settings.horizon.shader_pack.is_empty() {
                        self.open_shader_page(Page::ShaderOptions);
                        self.open_shader_options();
                    }
                }
                horizon_mods::UiRequest::ShaderPacks => self.open_shader_page(Page::ShaderPacks),
                horizon_mods::UiRequest::ModList => {
                    self.mods.screen = None;
                    if self.screen == Screen::Playing && self.title.is_none() {
                        self.capture(false);
                    }
                    self.open_options_at(self.title.is_some(), Page::ModMenu);
                }
                horizon_mods::UiRequest::ReloadShaderPack => {
                    self.pack_ids = None;
                    super::shader_packs::reload_shader_pack_job();
                    if let Some(renderer) = &mut self.renderer {
                        renderer.drop_shader_packs();
                    }
                }
            }
        }
    }
    /// The mod button of the title screen and pause menu: its mod and label.
    pub(super) fn mods_title_button(&self) -> Option<(usize, String)> {
        self.mods.mods.title_button()
    }
    /// The atlas rectangle of the icon of the mod with the title-screen button.
    pub(super) fn mod_button_icon(&mut self) -> Option<[f32; 4]> {
        let (owner, _) = self.mods.mods.title_button()?;
        let e = self.mods.mods.entries().get(owner)?;
        let icon = e.manifest.as_ref()?.icon.clone();
        if icon.is_empty() {
            return None;
        }
        let path = e.path.clone();
        self.model_texture(&path, &icon)
    }
    /// The Video Settings mod button: its mod hears the click.
    pub(super) fn mod_video_button(&mut self) {
        let Some((owner, _)) = self.mods.mods.video_button() else { return };
        self.mods.mods.ui_select(owner, horizon_mods::VIDEO_BUTTON, 0);
        self.show_mod_output();
    }
    /// The title screen's mod button: its mod hears the click, and may open its screen.
    pub(super) fn mod_title_button(&mut self) {
        let Some((owner, _)) = self.mods.mods.title_button() else { return };
        self.mods.title_click = Some(owner);
        self.mods.mods.ui_select(owner, horizon_mods::TITLE_BUTTON, 0);
        self.show_mod_output();
        self.mods.title_click = None;
    }
    fn open_shader_page(&mut self, page: Page) {
        let from = self.mods.settings_from.take();
        self.mods.screen = None;
        if self.screen == Screen::Playing && self.title.is_none() {
            self.capture(false);
        }
        match from {
            Some(from) => self.open_options_at(self.title.is_some(), from),
            None => self.open_options(self.title.is_some()),
        }
        self.ui.options.show(Page::ShaderPacks);
        self.settings.shader_packs =
            horizon_client::shader_pack_list::scan(&Self::shader_packs_dir());
        if page == Page::ShaderOptions {
            self.settings.shader_options = None;
            self.ui.options.show(Page::ShaderOptions);
        }
    }
    /// `/command` typed in chat: true when a mod owns it.
    pub(super) fn mod_command(&mut self, command: &str) -> bool {
        if !self.mods.mods.is_running() {
            return false;
        }
        // The command may follow a teleport in the same tick.
        let snapshot = self.mod_snapshot();
        self.mods.mods.set_snapshot(snapshot);
        if !self.mods.mods.command(command) {
            return false;
        }
        self.show_mod_output();
        true
    }
    /// A chat line the player received.
    pub(super) fn mods_chat(&mut self, line: &str) {
        self.mods.mods.chat(line);
        self.show_mod_output();
    }
    /// What mods showed: client mods' lines into the chat, server mods'
    /// to their players.
    pub(super) fn show_mod_output(&mut self) {
        self.route_mod_output(None);
    }
    /// [`Game::show_mod_output`], with the hub when it is out of `self.lan`.
    pub(super) fn route_mod_output(&mut self, mut host: Option<&mut LanHost>) {
        for output in self.mods.mods.drain() {
            let line = match output {
                Output::Chat { name, text } => {
                    let mut line = Text::literal(format!("[{name}] ")).with_color(text::GRAY);
                    line.push(Text::literal(text));
                    line
                }
                Output::Feedback { text, error: false } => Text::literal(text),
                Output::Feedback { text, error: true } => Text::literal(text).with_color(text::RED),
                // Vanilla's commands only: a mod running a mod command could loop.
                Output::Command { line } => {
                    let name = line.split(' ').next().unwrap_or_default();
                    if !vanilla_command(name) {
                        // Dropped.
                    } else if self.remote.is_some() {
                        // The server checks the player's permissions.
                        self.send_chat(&format!("/{line}"));
                    } else {
                        self.mods.commands.push(line);
                    }
                    continue;
                }
                Output::QuietCommands { mod_id, lines } => {
                    self.mod_quiet_commands(mod_id, lines);
                    continue;
                }
                Output::Stopped { name, reason } => {
                    // Its screen goes with it: the entry's run is gone, so
                    // `mod_ui_view` finds no widgets and the screen closes.
                    self.list_mods();
                    self.sync_mod_commands();
                    let line = format!("Mod {name} stopped: {reason}");
                    if self.options.dedicated.is_some() {
                        super::dedicated::log("WARN", &line);
                    }
                    Text::literal(line).with_color(text::RED)
                }
                server @ (Output::Tell { .. }
                | Output::Broadcast { .. }
                | Output::Reply { .. }
                | Output::SetBlocks { .. }) => {
                    self.route_server_output(host.as_deref_mut(), server);
                    continue;
                }
                Output::OpenScreen { owner } => {
                    self.open_mod_screen(owner, false);
                    continue;
                }
                Output::Shake { strength, seconds } => {
                    self.mods.shake.add(strength, seconds);
                    continue;
                }
                Output::PlaceRecipe { book_id, all } => {
                    self.mod_place_recipe(book_id, all);
                    continue;
                }
                Output::Give {
                    item,
                    count,
                    components,
                } => {
                    self.mod_give(&item, count, &components);
                    continue;
                }
                Output::CloseScreen { owner } => {
                    if self.mods.screen.as_ref().is_some_and(|s| s.owner == owner) {
                        self.close_mod_screen(false);
                    }
                    continue;
                }
            };
            self.add_mod_chat_line(line);
        }
        // A mod may set or clear its settings page in any call, not only
        // while loading: its row's Settings button follows.
        let rows = self.settings.mods.installed.iter_mut();
        for (row, e) in rows.zip(self.mods.mods.entries()) {
            row.has_settings = e.has_ui_settings();
        }
    }
    /// A mod's line in the window's chat; not through system_message: a mod
    /// must not hear its own lines.
    /// Written by this client, so the Chat option doesn't hide it.
    pub(super) fn add_mod_chat_line(&mut self, line: Text) {
        let focused = self.screen == Screen::Chat;
        self.ui.chat.add(
            &self.gui.atlas,
            line,
            None,
            horizon_client::chat::ChatSource::Client,
            self.ticks,
            focused,
        );
    }
    /// The mods' HUD panels, stacked with the extras in their corners.
    pub(super) fn draw_mod_panels(&self, batch: &mut HudBatch, corners: &mut Corners) {
        for panel in self.mods.mods.panels() {
            let corner = match panel.corner {
                horizon_mods::Corner::TopLeft => Corner::TopLeft,
                horizon_mods::Corner::TopRight => Corner::TopRight,
                horizon_mods::Corner::BottomLeft => Corner::BottomLeft,
                horizon_mods::Corner::BottomRight => Corner::BottomRight,
            };
            // F3 takes the top left, as it does from the extras.
            if self.debug && corner == Corner::TopLeft {
                continue;
            }
            let lines: Vec<(&str, u32)> = panel
                .lines
                .iter()
                .map(|l| (l.text.as_str(), l.color))
                .collect();
            extras_hud::draw_lines(batch, &self.gui.atlas, corners, corner, &lines);
        }
        // Capability `render`'s corner panels, placed the same way.
        for panel in self.mods.mods.corner_panels() {
            let corner = match panel.corner {
                horizon_mods::RenderCorner::TopLeft => Corner::TopLeft,
                horizon_mods::RenderCorner::TopRight => Corner::TopRight,
                horizon_mods::RenderCorner::BottomLeft => Corner::BottomLeft,
                horizon_mods::RenderCorner::BottomRight => Corner::BottomRight,
            };
            if self.debug && corner == Corner::TopLeft {
                continue;
            }
            let at = corners.place(corner, panel.width, panel.height);
            self.draw_shapes(batch, &panel.shapes, at);
        }
    }

    /// A `ui` screen's title and widgets as the mod last pushed them; None
    /// when the surface is gone (the mod stopped or closed it). Borrows
    /// `mods` only, so `self.mods.screen` may stay borrowed for the scrolls.
    fn mod_ui_view<'a>(mods: &'a Mods, ms: &ModScreen) -> Option<(&'a str, &'a [UiWidget])> {
        if ms.settings {
            let widgets = mods.settings_widgets(ms.owner);
            if widgets.is_empty() && mods.keys_of(ms.owner).is_empty() {
                return None;
            }
            let name = mods
                .entries()
                .get(ms.owner)
                .and_then(|e| e.manifest.as_ref())
                .map_or("Mod", |m| m.name.as_str());
            Some((name, widgets))
        } else {
            let s: &UiScreen = mods.screen(ms.owner)?;
            Some((s.title.as_str(), s.widgets.as_slice()))
        }
    }
    /// The keys a settings page lists first: the mod's own (`input`).
    fn mod_page_keys<'a>(_mods: &'a Mods, _ms: &ModScreen) -> &'a [ModKey] {
        &[]
    }
    /// A key row's label as Key Binds shows one, "Zoom: Z", marked
    /// "> Zoom: Z <" in yellow while it waits for the new key.
    fn mod_key_label(k: &ModKey, waiting: bool) -> Text {
        let value = format!("{}: {}", k.label, keybinds::display_name(&k.key).plain());
        if waiting {
            Text::literal(format!("> {value} <")).with_color(text::YELLOW)
        } else {
            Text::literal(value)
        }
    }
    /// The player bound the key row that waited to `key`: the mod's key
    /// follows it now and at the next start (`horizon-mods.txt`).
    fn bind_mod_key(&mut self, key: String) {
        let Some(ms) = self.mods.screen.as_mut() else {
            return;
        };
        let Some(name) = ms.binding.take() else {
            return;
        };
        let owner = ms.owner;
        if !self.mods.mods.rebind(owner, &name, &key) {
            return;
        }
        let Some(id) = self
            .mods
            .mods
            .entries()
            .get(owner)
            .and_then(|e| e.manifest.as_ref())
            .map(|m| m.id.clone())
        else {
            return;
        };
        self.settings
            .mods
            .keys
            .entry(id)
            .or_default()
            .insert(name, key);
        self.settings.save(&Self::options_path());
    }
    /// A key while a mod screen is up: true when a key row waited for
    /// it (Key Binds: Escape binds none).
    pub(super) fn mod_screen_key(&mut self, key: KeyCode) -> bool {
        if self
            .mods
            .screen
            .as_ref()
            .is_none_or(|ms| ms.binding.is_none())
        {
            return false;
        }
        let value = match key {
            KeyCode::Escape => None,
            k => keybinds::key_name(k),
        };
        self.bind_mod_key(value.unwrap_or("key.keyboard.unknown").to_owned());
        true
    }
    /// The layout the mod screen's widgets get at `h`: (widget, top,
    /// height), the column being `COLUMN` wide down the middle.
    /// Where each key row and widget sits on a page scrolled by `page`, and the
    /// furthest the page scrolls.
    fn mod_ui_layout(
        h: f32,
        keys: usize,
        widgets: &[UiWidget],
        page: f32,
    ) -> (Vec<f32>, Vec<(f32, f32, f32, f32)>, f32) {
        let key_rows = (0..keys)
            .map(|i| MOD_UI_TOP + i as f32 * MOD_UI_KEY_ROW)
            .collect::<Vec<_>>();
        let mut out = Vec::with_capacity(widgets.len());
        let top = MOD_UI_TOP + keys as f32 * MOD_UI_KEY_ROW;
        let (y, right) = match Self::mod_ui_split(widgets) {
            Some(at) => {
                let left = -MOD_UI_SPLIT_LEFT - MOD_UI_GAP / 2.0 + MOD_UI_SPLIT_SHIFT;
                Self::mod_ui_column(&widgets[..at], left, MOD_UI_SPLIT_LEFT, top, &mut out);
                out.push((0.0, top, 0.0, 0.0));
                let start = out.len();
                let x = MOD_UI_GAP / 2.0 + MOD_UI_SPLIT_SHIFT;
                let y = Self::mod_ui_column(&widgets[at + 1..], x, MOD_UI_SPLIT_RIGHT, top, &mut out);
                (y, start)
            }
            None => {
                let y = Self::mod_ui_column(widgets, -MOD_UI_WIDE / 2.0, MOD_UI_WIDE, top, &mut out);
                (y, 0)
            }
        };
        let max = (y - 6.0 - (h - MOD_UI_BOTTOM)).max(0.0);
        let page = page.clamp(0.0, max);
        (
            key_rows.into_iter().map(|y| y - page).collect(),
            out.into_iter()
                .enumerate()
                .map(|(i, (x, y, ww, wh))| (x, if i >= right { y - page } else { y }, ww, wh))
                .collect(),
            max,
        )
    }
    /// Where a `column` entity box splits the page as Mod Menu's is: the widgets before it stay
    /// in a left column, the ones after it scroll in the right one.
    fn mod_ui_split(widgets: &[UiWidget]) -> Option<usize> {
        widgets
            .iter()
            .position(|w| matches!(w, UiWidget::Entity(e) if e.entity == "column"))
    }
    /// The scrollbar's x beside the scrolling widgets, from the middle of the screen.
    fn mod_ui_bar_x(widgets: &[UiWidget]) -> f32 {
        if Self::mod_ui_split(widgets).is_some() {
            MOD_UI_GAP / 2.0 + MOD_UI_SPLIT_SHIFT + MOD_UI_SPLIT_RIGHT + 6.0
        } else {
            MOD_UI_WIDE / 2.0 + 6.0
        }
    }
    /// Stacks `widgets` down a column `wide` pixels wide at `x` from `y`: buttons and sliders two
    /// to a row, small entity boxes four to a row. Where the column ends.
    fn mod_ui_column(
        widgets: &[UiWidget],
        x: f32,
        wide: f32,
        mut y: f32,
        out: &mut Vec<(f32, f32, f32, f32)>,
    ) -> f32 {
        let half_w = ((wide - MOD_UI_GAP) / 2.0).floor();
        let mut left_open = false;
        let mut tile = 0usize;
        let mut tile_h = 0.0f32;
        for w in widgets {
            if let UiWidget::Entity(e) = w
                && e.height <= MOD_UI_TILE_MAX
            {
                let wh = e.height.max(24) as f32;
                if left_open {
                    y += 26.0;
                    left_open = false;
                }
                let tw = ((wide - 3.0 * MOD_UI_GAP / 2.0) / 4.0).floor();
                out.push((x + tile as f32 * (tw + MOD_UI_GAP / 2.0), y, tw, wh));
                tile_h = tile_h.max(wh);
                tile += 1;
                if tile == 4 {
                    y += tile_h + 6.0;
                    tile = 0;
                    tile_h = 0.0;
                }
                continue;
            }
            if tile > 0 {
                y += tile_h + 6.0;
                tile = 0;
                tile_h = 0.0;
            }
            let half = matches!(w, UiWidget::Button(_) | UiWidget::Slider(_));
            let wh = match w {
                UiWidget::Text(_) => 12.0,
                UiWidget::Item(_) | UiWidget::Button(_) | UiWidget::Slider(_) => 20.0,
                UiWidget::List(l) => (l.rows.len().min(6) as f32 * 12.0) + 8.0,
                UiWidget::Entity(e) => e.height as f32,
            };
            if half && left_open {
                out.push((x + wide - half_w, y - 26.0, half_w, wh));
                left_open = false;
                continue;
            }
            if half {
                out.push((x, y, half_w, wh));
                left_open = true;
            } else {
                out.push((x, y, wide, wh));
                left_open = false;
            }
            y += wh + 6.0;
        }
        if tile > 0 {
            y += tile_h + 6.0;
        }
        y
    }
    /// A row at `wy`, `wh` high, lies wholly inside the page's view (below the
    /// title, above Done).
    fn mod_ui_whole(h: f32, wy: f32, wh: f32) -> bool {
        wy >= MOD_UI_TOP - 0.5 && wy + wh <= h - MOD_UI_BOTTOM + 0.5
    }
    /// Any part of a row lies inside the page's view: drawn clipped to it.
    fn mod_ui_shown(h: f32, wy: f32, wh: f32) -> bool {
        wy + wh > MOD_UI_TOP && wy < h - MOD_UI_BOTTOM
    }
    /// A mod pushes its screen (`ui.open-screen`), or Options > Mods opens
    /// its settings page (`settings`, the mod's `installed` index).
    pub(super) fn open_mod_screen(&mut self, owner: usize, settings: bool) {
        // Only the settings page is drawn at the title (Options > Mods); a
        // mod's own screen waits for a world, unless its title button opened it.
        let from_title = self.mods.title_click.take() == Some(owner);
        if self.title.is_some() && !settings && !from_title {
            return;
        }
        let mut settings = settings;
        let mut back = false;
        if settings {
            self.mods.mods.ui_settings_opened(owner);
            if self.mods.mods.take_open_screen(owner) {
                settings = false;
                back = true;
            } else if horizon_mods::ui_request_pending() {
                self.show_mod_output();
                return;
            }
        }
        if settings {
            let keys = self.mods.mods.keys_of(owner);
            if self.mods.mods.settings_widgets(owner).is_empty() && keys.is_empty() {
                self.show_mod_output();
                return;
            }
        }
        self.mods.screen = Some(ModScreen {
            owner,
            settings,
            back,
            under: self.screen,
            scrolls: Vec::new(),
            binding: None,
            slider: None,
            page: 0.0,
            pan: None,
        });
        self.screen = Screen::Mod;
        if self.player.is_some() {
            self.capture(false);
        }
        self.show_mod_output();
    }
    /// Leaves the mod screen: back to Options for a settings page, to what
    /// it covered for a mod's own. `tell` is Esc/Done, heard as `closed`.
    pub(super) fn close_mod_screen(&mut self, tell: bool) {
        let Some(ms) = self.mods.screen.take() else {
            return;
        };
        if ms.settings || ms.back {
            let page = self.mods.settings_from.take().unwrap_or(Page::Mods);
            self.open_options_at(self.title.is_some(), page);
        } else {
            self.screen = ms.under;
            if self.screen == Screen::Playing && self.player.is_some() {
                self.capture(true);
            }
        }
        if tell {
            self.mods.mods.ui_closed(ms.owner);
            self.show_mod_output();
        }
    }
    /// The mod's screen or settings page: its title over widgets stacked
    /// down the middle, a Done button, Esc to leave (vanilla's menus).
    pub(super) fn draw_mod_screen(&mut self, batch: &mut HudBatch) {
        self.sync_player_cape();
        let Some(ms) = self.mods.screen.as_mut() else {
            // The screen was dropped while it was up: back to the world,
            // mouse held again like any return to Playing.
            self.screen = Screen::Playing;
            if self.player.is_some() {
                self.capture(true);
            }
            return;
        };
        let Some((title, widgets)) = Self::mod_ui_view(&self.mods.mods, ms) else {
            // The mod's surface is gone (it stopped or cleared it): leave
            // the screen like Done did, without `closed` â€” the mod ended it.
            self.close_mod_screen(false);
            return;
        };
        // Item icons draw in a second pass: `draw_item` needs `&mut self`,
        // so `atlas` may not stay borrowed across it.
        let mut items: Vec<(String, f32, f32)> = Vec::new();
        // Entity boxes too: the model is built from `&self` after the loop.
        let mut entities: Vec<([f32; 4], String, bool)> = Vec::new();
        let atlas = &self.gui.atlas;
        let in_world = self.player.is_some();
        if in_world {
            batch.fill(atlas, 0.0, 0.0, batch.width, batch.height, 0xA010_0010);
        } else {
            batch.fill(atlas, 0.0, 0.0, batch.width, batch.height, 0x7010_0010);
        }
        ui::menu_background(batch, atlas, 0.0, 0.0, batch.width, batch.height, in_world);
        ui::centered_text(
            batch,
            atlas,
            &Text::literal(title),
            0.0,
            15.0,
            batch.width,
            u32::MAX,
        );
        let cx = (batch.width / 2.0).floor();
        let x = cx - MOD_UI_COLUMN / 2.0;
        // A settings page starts with the mod's keys, as Key Binds rows.
        let keys = Self::mod_page_keys(&self.mods.mods, ms);
        let (key_rows, layout, max) =
            Self::mod_ui_layout(batch.height, keys.len(), widgets, ms.page);
        ms.page = ms.page.clamp(0.0, max);
        let view_h = batch.height;
        let clip_start = batch.vertices.len();
        for (i, k) in keys.iter().enumerate() {
            let wy = key_rows[i];
            if !Self::mod_ui_shown(view_h, wy, 20.0) {
                continue;
            }
            let hover = ui::hovered(self.cursor, x, wy, MOD_UI_COLUMN, 20.0);
            let waiting = ms.binding.as_deref() == Some(k.name.as_str());
            let label = Self::mod_key_label(k, waiting);
            ui::button(
                batch,
                atlas,
                x,
                wy,
                MOD_UI_COLUMN,
                20.0,
                &label,
                true,
                hover,
            );
        }
        let mut list_i = 0usize;
        for (w, &(dx, wy, col, wh)) in widgets.iter().zip(&layout) {
            let x = cx + dx;
            let whole_only = matches!(w, UiWidget::Entity(_) | UiWidget::Item(_));
            if !Self::mod_ui_shown(view_h, wy, wh) || (whole_only && !Self::mod_ui_whole(view_h, wy, wh)) {
                if matches!(w, UiWidget::List(_)) {
                    ms.scrolls.resize_with(list_i + 1, ui::Scroll::default);
                    list_i += 1;
                }
                continue;
            }
            match w {
                UiWidget::Button(b) => {
                    let hover = ui::hovered(self.cursor, x, wy, col, 20.0);
                    let label = Text::literal(b.label.clone());
                    ui::button(
                        batch,
                        atlas,
                        x,
                        wy,
                        col,
                        20.0,
                        &label,
                        true,
                        hover,
                    );
                }
                UiWidget::Slider(sl) => {
                    let hover = ui::hovered(self.cursor, x, wy, col, 20.0);
                    let label = Text::literal(sl.label.clone());
                    ui::slider(
                        batch,
                        atlas,
                        x,
                        wy,
                        col,
                        &label,
                        sl.position as f32 / sl.steps.max(1) as f32,
                        hover,
                        true,
                        ms.slider == Some(sl.id),
                    );
                }
                UiWidget::Text(t) => {
                    let color = if t.color == 0 {
                        u32::MAX
                    } else {
                        t.color | 0xFF00_0000
                    };
                    ui::centered_text(
                        batch,
                        atlas,
                        &Text::literal(t.text.clone()),
                        x,
                        wy + 1.0,
                        col,
                        color,
                    );
                }
                UiWidget::Entity(e) if e.entity == "column" => {}
                UiWidget::Entity(e) => {
                    let hover = ui::hovered(self.cursor, x, wy, col, wh);
                    batch.fill(atlas, x, wy, col, wh, if hover { 0x9057_197F } else { 0x6000_0000 });
                    if hover {
                        let rim = 0xFFB9_8CF2;
                        batch.fill(atlas, x, wy, col, 1.0, rim);
                        batch.fill(atlas, x, wy + wh - 1.0, col, 1.0, rim);
                        batch.fill(atlas, x, wy, 1.0, wh, rim);
                        batch.fill(atlas, x + col - 1.0, wy, 1.0, wh, rim);
                    }
                    entities.push(([x, wy, x + col, wy + wh], e.entity.clone(), e.follow_cursor));
                }
                UiWidget::Item(i) => {
                    if self.res.items().get(&i.item).is_some() {
                        items.push((i.item.clone(), x + 2.0, wy + 2.0));
                    } else {
                        batch.text(atlas, "?", x + 6.0, wy + 6.0, u32::MAX, true);
                    }
                    let name = if i.name.is_empty() {
                        i.item.rsplit(':').next().unwrap_or(&i.item).to_string()
                    } else {
                        i.name.clone()
                    };
                    batch.text_styled(
                        atlas,
                        &Text::literal(name),
                        x + 22.0,
                        wy + 6.0,
                        u32::MAX,
                        true,
                    );
                }
                UiWidget::List(l) => {
                    ms.scrolls.resize_with(list_i + 1, ui::Scroll::default);
                    let scroll = ms.scrolls[list_i];
                    list_i += 1;
                    let rows_top = wy + 4.0;
                    let rows_bottom = wy + wh - 4.0;
                    batch.fill(atlas, x, wy, col, wh, 0x8000_0000);
                    for (b, yy) in [(x, wy), (x, wy + wh - 1.0)] {
                        batch.fill(atlas, b, yy, col, 1.0, 0xFF50_5050);
                    }
                    for (xx, yy) in [(x, wy), (x + col - 1.0, wy)] {
                        batch.fill(atlas, xx, yy, 1.0, wh, 0xFF50_5050);
                    }
                    let content = l.rows.len() as f32 * 12.0;
                    draw_list_rows(
                        batch,
                        atlas,
                        self.cursor,
                        &l.rows,
                        l.selected,
                        scroll.amount,
                        (x, wy, wh, col),
                    );
                    ui::scrollbar(
                        batch,
                        atlas,
                        x + col - 5.0,
                        rows_top,
                        rows_bottom,
                        content,
                        scroll.amount,
                    );
                }
            }
        }
        batch.clip_since(clip_start, 0.0, MOD_UI_TOP, batch.width, view_h - MOD_UI_BOTTOM);
        if max > 0.0 {
            let content = view_h - MOD_UI_BOTTOM - MOD_UI_TOP + max;
            ui::scrollbar(
                batch,
                atlas,
                cx + Self::mod_ui_bar_x(widgets),
                MOD_UI_TOP,
                view_h - MOD_UI_BOTTOM,
                content,
                ms.page,
            );
        }
        let hover = ui::hovered(self.cursor, cx - 100.0, batch.height - 27.0, 200.0, 20.0);
        ui::button(
            batch,
            atlas,
            cx - 100.0,
            batch.height - 27.0,
            200.0,
            20.0,
            &Text::literal(horizon_client::lang::get("gui.done")),
            true,
            hover,
        );
        let owner_path = self
            .mods
            .screen
            .as_ref()
            .and_then(|ms| self.mods.mods.entries().get(ms.owner))
            .map(|e| e.path.clone());
        for ([x0, y0, x1, y1], entity, follow) in entities {
            if let Some(file) = entity.strip_prefix("cape:") {
                let uv = owner_path
                    .as_ref()
                    .filter(|_| !file.contains("..") && file.ends_with(".png"))
                    .and_then(|p| self.model_texture(p, &format!("models/{file}")));
                if let Some(uv) = uv {
                    self.draw_cape_box(batch, [x0, y0, x1, y1], uv);
                }
                continue;
            }
            self.draw_entity_box(batch, x0, y0, x1, y1, &entity, follow);
        }
        for (id, ix, iy) in items {
            let stack = ItemStack::new(id.as_str(), 1);
            self.draw_item(batch, &stack, ix, iy);
        }
    }
    /// A click on the mod screen: Done leaves it, a `list` row is the mod's
    /// `ui-events::select`.
    pub(super) fn mod_screen_click(&mut self, button: MouseButton) {
        // Key Binds: a mouse button pressed while waiting binds it.
        if self
            .mods
            .screen
            .as_ref()
            .is_some_and(|ms| ms.binding.is_some())
        {
            self.bind_mod_key(keybinds::mouse_name(button));
            return;
        }
        if button != MouseButton::Left {
            return;
        }
        let (w, h) = self.gui_size();
        let cursor = self.cursor;
        if ui::hovered(cursor, (w / 2.0).floor() - 100.0, h - 27.0, 200.0, 20.0) {
            self.audio.play_button_click();
            self.close_mod_screen(true);
            return;
        }
        let Some(ms) = self.mods.screen.as_ref() else {
            return;
        };
        let Some((_, widgets)) = Self::mod_ui_view(&self.mods.mods, ms) else {
            return;
        };
        let owner = ms.owner;
        let page = ms.page;
        let cx = (w / 2.0).floor();
        let x = cx - MOD_UI_COLUMN / 2.0;
        // A key row waits for the key to bind.
        let keys = Self::mod_page_keys(&self.mods.mods, ms);
        let (key_rows, layout, page_max) = Self::mod_ui_layout(h, keys.len(), widgets, page);
        let picked = (0..keys.len()).find(|&i| {
            let wy = key_rows[i];
            Self::mod_ui_shown(h, wy, 20.0)
                && cursor.1 >= MOD_UI_TOP
                && cursor.1 < h - MOD_UI_BOTTOM
                && ui::hovered(cursor, x, wy, MOD_UI_COLUMN, 20.0)
        });
        if let Some(i) = picked {
            let name = keys[i].name.clone();
            self.audio.play_button_click();
            if let Some(ms) = self.mods.screen.as_mut() {
                ms.binding = Some(name);
            }
            return;
        }
        let mut list_i = 0usize;
        // The `ui_select` call needs `&mut`: find the pick first.
        let mut hit = None;
        let in_band = cursor.1 >= MOD_UI_TOP && cursor.1 < h - MOD_UI_BOTTOM;
        for (w, &(dx, wy, col, wh)) in widgets.iter().zip(&layout) {
            let x = cx + dx;
            if !in_band || !Self::mod_ui_shown(h, wy, wh) {
                if matches!(w, UiWidget::List(_)) {
                    list_i += 1;
                }
                continue;
            }
            // Shift + click: the mod's default, for the widgets it named (`ui.set-resets`).
            let reset_id = match w {
                UiWidget::Slider(sl) => Some(sl.id),
                UiWidget::Button(b) => Some(b.id),
                _ => None,
            };
            if let Some(id) = reset_id
                && ui::hovered(cursor, x, wy, col, 20.0)
                && self.shift_held()
                && self.mods.mods.ui_resets(owner, id)
            {
                self.audio.play_button_click();
                self.mods.mods.ui_select(owner, id, horizon_mods::RESET_ROW);
                self.show_mod_output();
                return;
            }
            if let UiWidget::Slider(sl) = w
                && ui::hovered(cursor, x, wy, col, 20.0)
            {
                let owner = self.mods.screen.as_ref().map_or(0, |ms| ms.owner);
                if let Some(ms) = self.mods.screen.as_mut() {
                    ms.slider = Some(sl.id);
                }
                let position = Self::slider_position(cursor.0, x, col, sl.steps);
                self.mods.mods.ui_select(owner, sl.id, position);
                self.show_mod_output();
                return;
            }
            // An entity box is `select(id, 0)` too.
            if let UiWidget::Entity(e) = w
                && ui::hovered(cursor, x, wy, col, wh)
            {
                self.audio.play_button_click();
                let owner = self.mods.screen.as_ref().map_or(0, |ms| ms.owner);
                self.mods.mods.ui_select(owner, e.id, 0);
                self.show_mod_output();
                return;
            }
            // A button is `select(id, 0)`.
            if let UiWidget::Button(b) = w
                && ui::hovered(cursor, x, wy, col, 20.0)
            {
                self.audio.play_button_click();
                let owner = self.mods.screen.as_ref().map_or(0, |ms| ms.owner);
                self.mods.mods.ui_select(owner, b.id, 0);
                self.show_mod_output();
                return;
            }
            let UiWidget::List(l) = w else { continue };
            let scroll = ms.scrolls.get(list_i).copied().unwrap_or_default();
            list_i += 1;
            let row = ((cursor.1 - (wy + 4.0) + scroll.amount) / 12.0) as i64;
            if ui::hovered(cursor, x + 1.0, wy + 1.0, col - 7.0, wh - 2.0)
                && row >= 0
                && (row as usize) < l.rows.len()
            {
                hit = Some((l.id, row as u32));
                break;
            }
        }
        if let Some((id, row)) = hit {
            self.audio.play_button_click();
            self.mods.mods.ui_select(owner, id, row);
            self.show_mod_output();
        } else if let Some(ms) = self.mods.screen.as_mut() {
            // Only the scrollbar drags the page; the thumb follows the cursor.
            let bar_x = cx + Self::mod_ui_bar_x(widgets);
            if page_max > 0.0
                && cursor.0 >= bar_x - 3.0
                && cursor.0 < bar_x + 9.0
                && cursor.1 >= MOD_UI_TOP
                && cursor.1 < h - MOD_UI_BOTTOM
            {
                ms.pan = Some((cursor.1, ms.page));
            }
        }
    }
    /// The slider position under cursor column `cx` of a track at `x`.
    fn slider_position(cx: f32, x: f32, col: f32, steps: u32) -> u32 {
        let value = ((cx - (x + 4.0)) / (col - 8.0)).clamp(0.0, 1.0);
        (value * steps as f32).round() as u32
    }
    /// A drag moved the cursor: the dragged slider follows it.
    pub(super) fn mod_screen_drag(&mut self) {
        let cursor_y = self.cursor.1;
        if let Some(ms) = self.mods.screen.as_ref()
            && let Some((from, start)) = ms.pan
        {
            let (_, h) = self.gui_size();
            let keys = Self::mod_page_keys(&self.mods.mods, ms).len();
            let max = Self::mod_ui_view(&self.mods.mods, ms)
                .map_or(0.0, |(_, widgets)| Self::mod_ui_layout(h, keys, widgets, ms.page).2);
            let track = (h - MOD_UI_BOTTOM - MOD_UI_TOP).max(1.0);
            let content = track + max;
            let bar = ui::scrollbar_thumb(MOD_UI_TOP, h - MOD_UI_BOTTOM, content, start).map_or(track, |(_, b)| b);
            let per = if track - bar > 0.0 { max / (track - bar) } else { 0.0 };
            if let Some(ms) = self.mods.screen.as_mut() {
                ms.page = (start + (cursor_y - from) * per).clamp(0.0, max);
            }
            return;
        }
        let Some(ms) = self.mods.screen.as_ref() else {
            return;
        };
        let Some(id) = ms.slider else {
            return;
        };
        let owner = ms.owner;
        let Some((_, widgets)) = Self::mod_ui_view(&self.mods.mods, ms) else {
            return;
        };
        let (w, h) = self.gui_size();
        let cx = (w / 2.0).floor();
        let (_, layout, _) = Self::mod_ui_layout(h, 0, widgets, ms.page);
        let steps = widgets.iter().zip(&layout).find_map(|(w, l)| match w {
            UiWidget::Slider(s) if s.id == id => Some((s.steps, s.position, *l)),
            _ => None,
        });
        let Some((steps, current, (dx, _, col, _))) = steps else {
            return;
        };
        let position = Self::slider_position(self.cursor.0, cx + dx, col, steps);
        if position != current {
            self.mods.mods.ui_select(owner, id, position);
            self.show_mod_output();
        }
    }
    /// The mouse button came up: no slider is held.
    pub(super) fn mod_screen_release(&mut self) {
        if let Some(ms) = self.mods.screen.as_mut() {
            ms.slider = None;
            ms.pan = None;
        }
    }
    /// The wheel over a `list` scrolls it.
    pub(super) fn mod_screen_scroll(&mut self, delta: f32) {
        let (w, h) = self.gui_size();
        let cursor = self.cursor;
        let Some(ms) = self.mods.screen.as_mut() else {
            return;
        };
        let Some((_, widgets)) = Self::mod_ui_view(&self.mods.mods, ms) else {
            return;
        };
        let cx = (w / 2.0).floor();
        let keys = Self::mod_page_keys(&self.mods.mods, ms).len();
        let (_, layout, max) = Self::mod_ui_layout(h, keys, widgets, ms.page);
        let mut list_i = 0usize;
        for (w, &(dx, wy, col, wh)) in widgets.iter().zip(&layout) {
            let UiWidget::List(l) = w else { continue };
            let over =
                Self::mod_ui_shown(h, wy, wh) && ui::hovered(cursor, cx + dx, wy, col, wh);
            ms.scrolls.resize_with(list_i + 1, ui::Scroll::default);
            if over {
                let content = l.rows.len() as f32 * 12.0;
                ms.scrolls[list_i].wheel(delta, 12.0, wy + 4.0, wy + wh - 4.0, content);
                return;
            }
            list_i += 1;
        }
        ms.page = (ms.page - delta * MOD_UI_KEY_ROW).clamp(0.0, max);
    }
}

/// A mod screen's `list` rows in its frame at (`x`, `wy`), `wh` high, scrolled by `scroll`: the
/// rows in view, the hovered one lit and the selected one marked. As AbstractSelectionList draws
/// its entries inside `enableScissor`, they are cut at the frame's inside: the row a scroll shows
/// in part, and the one below a full view, never draw over the frame or below it.
#[cfg(feature = "mods")]
fn draw_list_rows(
    batch: &mut HudBatch,
    atlas: &horizon_client::hud::HudAtlas,
    cursor: (f32, f32),
    rows: &[String],
    selected: Option<u32>,
    scroll: f32,
    (x, wy, wh, col): (f32, f32, f32, f32),
) {
    let rows_top = wy + 4.0;
    let rows_bottom = wy + wh - 4.0;
    let first = (scroll / 12.0) as usize;
    let last = (first + ((rows_bottom - rows_top) / 12.0).ceil() as usize + 1).min(rows.len());
    let start = batch.vertices.len();
    for (vi, ri) in (first..last).enumerate() {
        let ry = rows_top + vi as f32 * 12.0 - (scroll % 12.0);
        if ui::hovered(cursor, x, ry, col - 6.0, 12.0) {
            batch.fill(atlas, x + 1.0, ry, col - 8.0, 12.0, 0x20FF_FFFF);
        }
        if selected == Some(ri as u32) {
            batch.fill(atlas, x + 1.0, ry, 3.0, 12.0, 0xFFFF_FFFF);
        }
        batch.text_styled(
            atlas,
            &Text::literal(rows[ri].clone()),
            x + 6.0,
            ry + 2.0,
            u32::MAX,
            true,
        );
    }
    batch.clip_since(
        start,
        x + 1.0,
        wy + 1.0,
        x + col - 1.0,
        wy + wh - 1.0,
    );
}

#[cfg(feature = "mods")]
const MOD_UI_TOP: f32 = 40.0;
#[cfg(feature = "mods")]
const MOD_UI_BOTTOM: f32 = 34.0;
#[cfg(feature = "mods")]
const MOD_UI_COLUMN: f32 = 200.0;
#[cfg(feature = "mods")]
const MOD_UI_WIDE: f32 = 310.0;
/// Entity boxes this high or lower are tiles, four to a row.
const MOD_UI_TILE_MAX: u32 = 64;
/// A split page: the fixed left column's width, the scrolling right one's, and how far the pair
/// sits right of the middle.
#[cfg(feature = "mods")]
const MOD_UI_SPLIT_LEFT: f32 = 150.0;
#[cfg(feature = "mods")]
const MOD_UI_SPLIT_RIGHT: f32 = 230.0;
#[cfg(feature = "mods")]
const MOD_UI_SPLIT_SHIFT: f32 = -40.0;
#[cfg(feature = "mods")]
const MOD_UI_GAP: f32 = 10.0;
/// A key row's height with its gap, as the widgets below it space out.
#[cfg(feature = "mods")]
const MOD_UI_KEY_ROW: f32 = 26.0;

#[cfg(all(test, feature = "mods"))]
mod tests {
    use super::*;
    use horizon_client::hud::HudAtlas;

    #[test]
    fn list_rows_stay_inside_the_frame_scrolled_or_not() {
        let atlas =
            HudAtlas::build(std::path::Path::new("/nonexistent/horizon-test-assets")).unwrap();
        // #3237: 22 rows in a frame of 6 (mod_ui_layout's height for a long list). The 7th row
        // (index 6), drawn for a scroll that shows it in part, is selected and under the
        // cursor, so its fills reach past the frame unless they are cut.
        let rows: Vec<String> = (0..22).map(|i| format!("Row {i}")).collect();
        let (x, wy, wh) = (100.0, 40.0, 6.0 * 12.0 + 8.0);
        for scroll in [0.0, 6.0, 30.0] {
            let mut batch = HudBatch::new([640, 480]);
            let seventh_row = wy + 4.0 + 6.0 * 12.0 - scroll % 12.0;
            draw_list_rows(
                &mut batch,
                &atlas,
                (x + 10.0, seventh_row + 2.0),
                &rows,
                Some(6 + (scroll / 12.0) as u32),
                scroll,
                (x, wy, wh, MOD_UI_COLUMN),
            );
            assert!(!batch.vertices.is_empty(), "scroll {scroll}: rows drawn");
            let (top, bottom) = batch
                .vertices
                .iter()
                .map(|v| v.pos[1])
                .fold((f32::MAX, f32::MIN), |(lo, hi), y| (lo.min(y), hi.max(y)));
            assert!(
                top >= wy + 1.0 && bottom <= wy + wh - 1.0,
                "scroll {scroll}: rows span {top}..{bottom}, the frame's inside is {}..{}",
                wy + 1.0,
                wy + wh - 1.0
            );
        }
    }
}

#[cfg(test)]
fn not_built(
    dir: &std::path::Path,
    built: &str,
    rows: &[mod_list::Installed],
) -> Vec<mod_list::Installed> {
    let info: serde_json::Value = serde_json::from_str(built).unwrap_or_default();
    let in_build = |id: &str| {
        info["mods"]
            .as_array()
            .is_some_and(|mods| mods.iter().any(|m| m["id"] == id))
    };
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|d| d.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    paths.retain(|p| p.extension().is_some_and(|e| e == "hnmod"));
    paths.sort();
    let mut out: Vec<mod_list::Installed> = Vec::new();
    for path in paths {
        let Ok(bundle) = horizon_mods::index::native::Bundle::read(&path) else {
            continue;
        };
        let m = bundle.manifest;
        let listed = rows.iter().chain(&out).any(|row| row.id == m.id);
        if !listed && !in_build(&m.id) {
            out.push(mod_list::Installed {
                title: format!("{} {}", m.id, m.version),
                id: m.id,
                description: String::new(),
                problem: Some(mod_list::NOT_BUILT.to_owned()),
                dev: false,
                native: None,
                has_settings: false,
                details: Default::default(),
            });
        }
    }
    out
}

/// A mod gave `block` block entity `id`, but the block's template has none,
/// so breaking it would never remove it: the block gets none.
pub(super) fn warn_no_block_entity(block: &str, id: &str) {
    log::warn!("{block}: no block entity {id}, its template has none (copy a chest or furnace)");
}

#[cfg(test)]
mod native_bundle_tests {
    use super::*;
    use horizon_client::mod_list::Installed;
    use std::io::Write;

    /// A minimal `.hnmod` the registry accepts: its manifest and its mixin's.
    fn hnmod(dir: &std::path::Path, id: &str) {
        let horizon = "a".repeat(40);
        let manifest = serde_json::json!({
            "schema": 1, "id": id, "version": "0.3.0", "game": "26.3",
            "horizon": horizon, "mixins": "b".repeat(40),
        });
        let mixin = format!(
            "[mod]\nid = \"{id}\"\nversion = \"0.3.0\"\n[compatibility]\nhorizon_revision = \"{horizon}\"\n"
        );
        let file = std::fs::File::create(dir.join(format!("{id}.hnmod"))).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("horizon-native.json", options).unwrap();
        zip.write_all(manifest.to_string().as_bytes()).unwrap();
        zip.start_file(format!("mixins/{id}/horizon-mixin.toml"), options)
            .unwrap();
        zip.write_all(mixin.as_bytes()).unwrap();
        zip.finish().unwrap();
    }

    #[test]
    fn native_mods_installed_but_not_built_get_a_row_saying_so() {
        let dir = std::env::temp_dir().join(format!("horizon-not-built-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for id in ["veinminer", "zoom", "minimap"] {
            hnmod(&dir, id);
        }
        std::fs::write(dir.join("notes.txt"), "not a mod").unwrap();
        std::fs::write(dir.join("broken.hnmod"), "not a zip").unwrap();
        // A release: none is built in; the page lists them, sorted, instead of "No mods".
        let rows = not_built(&dir, "", &[]);
        let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["minimap", "veinminer", "zoom"]);
        assert_eq!(rows[1].title, "veinminer 0.3.0");
        assert_eq!(rows[1].problem.as_deref(), Some(mod_list::NOT_BUILT));
        assert!(rows.iter().all(|r| r.native.is_none() && !r.dev));
        // A native build with zoom, and minimap's row already listed by its own hook.
        let built =
            r#"{"base":"nightly-2026-10-30-1da354b","mods":[{"id":"zoom","version":"0.3.0"}]}"#;
        let listed = Installed {
            id: "minimap".into(),
            ..rows[0].clone()
        };
        let rows = not_built(&dir, built, &[listed]);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "veinminer");
        // No folder: no rows.
        assert!(not_built(&dir.join("missing"), "", &[]).is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
