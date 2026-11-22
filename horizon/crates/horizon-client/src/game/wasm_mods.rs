//! Horizon's wasm mods (`.hmod` with api "0.2" and a `[wasm]` table), run by
//! `pojarush-wasm-mods` on its guest thread: their rows on the Mods page (a page of
//! buttons, as native mods have), their HUD boxes, frames, clicks and settings.
use super::Game;
use horizon_client::{extras_hud::Corners, hud::HudBatch};
use std::time::Duration;
use winit::event::MouseButton;

#[cfg(not(feature = "mods"))]
impl Game<'_> {
    pub(super) fn start_wasm_mods(&mut self) {}
    pub(super) fn wasm_hud(&mut self, _batch: &mut HudBatch, _corners: &mut Corners) {}
    pub(super) fn wasm_frame(&mut self, _elapsed: Duration) {}
    pub(super) fn wasm_tick(&mut self) {}
    pub(super) fn wasm_mouse(&mut self, _button: MouseButton, _pressed: bool) {}
    pub(super) fn wasm_resources_reloaded(&mut self) {}
    pub(super) fn wasm_mod_rows(&mut self) {}
    pub(super) fn wasm_settings_click(&mut self, _id: &str, _button: usize) -> bool {
        false
    }
    pub(super) fn wasm_settings_buttons(&mut self, _id: &str) -> Option<Vec<String>> {
        None
    }
}

#[cfg(feature = "mods")]
mod running {
    use super::*;
    use crate::game::{Screen, execution_role::ExecutionRole};
    use horizon_client::{
        extras::Corner,
        hud::rgba,
        mod_list::{self, Installed, ModDetails, NativePage},
        text::Text,
    };
    use pojarush_wasm_mods::{BlockDef, Crosshair, DrawCmd, EntityView, Registries, View, WasmMods};

    const MAPPINGS: [&str; 9] = [
        "key.forward",
        "key.left",
        "key.back",
        "key.right",
        "key.jump",
        "key.sneak",
        "key.sprint",
        "key.attack",
        "key.use",
    ];

    fn translate(key: &str) -> Option<String> {
        horizon_client::lang::has(key).then(|| horizon_client::lang::get(key))
    }

    impl Game<'_> {
        /// Once, for the window's game: the wasm packages in `horizon-mods/`, started on their
        /// guest thread (the first start compiles them; later starts read the cache).
        pub(in crate::game) fn start_wasm_mods(&mut self) {
            if self.mods.wasm_started || self.options.execution_role != ExecutionRole::Client {
                return;
            }
            self.mods.wasm_started = true;
            let dir = Self::options_path().with_file_name(mod_list::DIR);
            let game_dir = Self::options_path()
                .parent()
                .map(std::path::Path::to_path_buf)
                .unwrap_or_default();
            let off: Vec<String> = self.settings.mods.disabled.iter().cloned().collect();
            let cache = crate::game::packs::cache_root().join("wasm-modules");
            let registries = self.wasm_registries();
            self.mods.wasm = WasmMods::start(&dir, &game_dir, &cache, &off, registries);
        }

        /// The game's blocks, items and entity types, for the mods' registry keys.
        fn wasm_registries(&self) -> Registries {
            let registry = self.pack.assets.registry;
            let facts = self.res.facts();
            let mut blocks: Vec<BlockDef> = Vec::new();
            for i in 0..registry.len() {
                let id = horizon_world::registry::BlockStateId(i as u32);
                let Some(state) = registry.state(id) else {
                    continue;
                };
                let (requires_tool, entity) = facts
                    .states
                    .get(i)
                    .map_or((false, false), |f| (f.requires_tool, f.block_entity));
                if blocks.last().is_none_or(|b| b.name != state.name) {
                    let default = registry
                        .default_state(&state.name)
                        .map_or(0, |d| d.0.saturating_sub(i as u32) as u16);
                    blocks.push(BlockDef {
                        name: state.name.clone(),
                        states: Vec::new(),
                        default,
                        requires_tool: Vec::new(),
                        has_block_entity: entity,
                    });
                }
                let block = blocks.last_mut().expect("pushed above");
                block.states.push(state.properties.clone());
                block.requires_tool.push(requires_tool);
            }
            let items = self.items().registry_order.iter().map(|i| i.to_string()).collect();
            let mut entity_types: Vec<String> = self.res.mob_types().keys().cloned().collect();
            let export = std::fs::read(self.options.resources.join("command-registries.json"))
                .ok()
                .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok());
            if let Some(ids) = export
                .as_ref()
                .and_then(|e| e.pointer("/by_registry/minecraft:entity_type/ids"))
                .and_then(|v| v.as_array())
            {
                entity_types.extend(ids.iter().filter_map(|v| v.as_str().map(str::to_owned)));
            }
            entity_types.push("minecraft:player".into());
            entity_types.sort();
            entity_types.dedup();
            Registries { blocks, items, entity_types }
        }

        /// The local player's id among the entities a wasm mod reads.
        const LOCAL_PLAYER: u32 = u32::MAX - 1;

        /// What the crosshair is on, and the entities mods may read.
        fn wasm_world(&self) -> (Option<Crosshair>, Vec<EntityView>, Option<u32>) {
            let mut entities = Vec::new();
            let mut local = None;
            if let Some(p) = &self.player {
                let main_hand = self.held().map(|s| (s.item.to_string(), s.count));
                let pos = [p.position.x, p.position.y, p.position.z];
                let eye = self.aim().map_or(pos, |(eye, _)| eye);
                entities.push(EntityView {
                    id: Self::LOCAL_PLAYER,
                    kind: "minecraft:player".into(),
                    name: {
                        static NAME: std::sync::OnceLock<String> = std::sync::OnceLock::new();
                        NAME.get_or_init(|| self.options.player_name()).clone()
                    },
                    position: pos,
                    eye,
                    uuid: 0,
                    health: Some((p.health.health(), 20.0)),
                    main_hand,
                });
                local = Some(Self::LOCAL_PLAYER);
            }
            let registry = self.pack.assets.registry;
            let mob = self.mob_hit().filter(|&(_, d)| self.before_block(d));
            let crosshair = if let Some((i, _)) = mob {
                let m = &self.mobs[i];
                let name = m.custom_name.clone().unwrap_or_else(|| {
                    let key = format!("entity.{}", m.kind.id.replacen(':', ".", 1));
                    horizon_client::lang::get(&key)
                });
                let pos = [m.position.x, m.position.y, m.position.z];
                entities.push(EntityView {
                    id: m.id,
                    kind: m.kind.id.clone(),
                    name,
                    position: pos,
                    eye: [pos[0], pos[1] + m.kind.eye_height, pos[2]],
                    uuid: 0,
                    health: Some((m.health, horizon_client::mobs::finalize::max_health(m))),
                    main_hand: None,
                });
                Some(Crosshair::Entity { id: m.id, point: pos })
            } else {
                self.pick().and_then(|hit| {
                    let state = self.world.block(hit.pos[0], hit.pos[1], hit.pos[2]);
                    let name = registry.state(state)?.name.clone();
                    let first = registry.default_state(&name)?;
                    let mut start = first.0;
                    while start > 0
                        && registry
                            .state(horizon_world::registry::BlockStateId(start - 1))
                            .is_some_and(|s| s.name == name)
                    {
                        start -= 1;
                    }
                    Some(Crosshair::Block {
                        pos: hit.pos,
                        block: name,
                        ordinal: state.0.saturating_sub(start) as u16,
                        face: hit.face,
                        point: hit.point,
                    })
                })
            };
            (crosshair, entities, local)
        }

        fn wasm_view(&mut self) -> View {
            let mappings = MAPPINGS
                .iter()
                .filter_map(|&name| {
                    let (down, label) = self.native_mapping(name)?;
                    Some((name, label, down))
                })
                .collect();
            if self.mods.font_widths.is_empty() {
                let atlas = &self.gui.atlas;
                self.mods.font_widths = (0..0x250u32)
                    .map(|c| char::from_u32(c).map_or(6.0, |c| atlas.glyph_cell(c).1))
                    .collect();
            }
            let (crosshair, entities, local_player) = self.wasm_world();
            View {
                millis: self.started.elapsed().as_millis() as u64,
                playing: self.native_playing() && self.screen == Screen::Playing,
                mappings,
                corners_shown: Corner::ALL.map(|c| self.native_corner_shown(c)),
                settings_root: Self::options_path()
                    .with_file_name(mod_list::DIR)
                    .join(".horizon-mod-data"),
                off: self.settings.mods.disabled.iter().cloned().collect(),
                cursor: [0.0, 0.0],
                widths: self.mods.font_widths.clone(),
                language: horizon_client::lang::code(),
                translate: Some(translate),
                crosshair,
                entities,
                local_player,
                pressed: Vec::new(),
                paused: self.pauses_game(),
            }
        }

        /// After the native mods' boxes: each wasm mod's, below what the corners hold.
        pub(in crate::game) fn wasm_hud(&mut self, batch: &mut HudBatch, corners: &mut Corners) {
            if !self.mods.wasm.is_running() {
                return;
            }
            let view = self.wasm_view();
            let (w, h, used) = corners.parts();
            let (mut cmds, used) = self.mods.wasm.render_gui_post(view.clone(), (w, h), used);
            *corners = Corners::from_parts(w, h, used);
            if view.playing {
                cmds.extend(self.mods.wasm.render_gui_layers(view, (w, h)));
            }
            self.wasm_draw(batch, cmds);
        }

        /// The wasm mods' drawing, in order.
        fn wasm_draw(&self, batch: &mut HudBatch, cmds: Vec<DrawCmd>) {
            let atlas = &self.gui.atlas;
            for cmd in cmds {
                match cmd {
                    DrawCmd::Fill { rect: [x, y, w, h], argb } => {
                        let [a, r, g, b] = argb.to_be_bytes();
                        batch.fill(atlas, x, y, w, h, rgba(r, g, b, a));
                    }
                    DrawCmd::Gradient { rect: [x, y, w, h], top, bottom } => {
                        let rows = h.round().max(1.0) as u32;
                        let step = h / rows as f32;
                        for i in 0..rows {
                            let t = if rows > 1 { i as f32 / (rows - 1) as f32 } else { 0.0 };
                            let mix = |shift: u32| {
                                let a = ((top >> shift) & 0xFF) as f32;
                                let b = ((bottom >> shift) & 0xFF) as f32;
                                (a + (b - a) * t).round() as u8
                            };
                            batch.fill(atlas, x, y + i as f32 * step, w, step, rgba(mix(16), mix(8), mix(0), mix(24)));
                        }
                    }
                    DrawCmd::Text { text, at: [x, y], argb, shadow, .. } => {
                        let [a, r, g, b] = argb.to_be_bytes();
                        let a = if a == 0 { 0xFF } else { a };
                        batch.text_styled(atlas, &Text::literal(text), x, y, rgba(r, g, b, a), shadow);
                    }
                    DrawCmd::Item { id, at: [x, y], .. } => {
                        let stack = horizon_items::items::ItemStack::new(&id, 1);
                        self.draw_item_model(batch, &stack, x, y);
                    }
                }
            }
        }

        /// Once a client tick, before it runs.
        pub(in crate::game) fn wasm_tick(&mut self) {
            if self.mods.wasm.is_running() {
                let view = self.wasm_view();
                self.mods.wasm.client_tick(view);
            }
        }

        /// Once a frame.
        pub(in crate::game) fn wasm_frame(&mut self, elapsed: Duration) {
            let status: Vec<Option<String>> =
                self.mods.wasm.entries().into_iter().map(|e| e.problem).collect();
            if status != self.mods.wasm_status {
                self.mods.wasm_status = status;
                self.list_native_mods();
            }
            if self.mods.wasm.is_running() {
                let view = self.wasm_view();
                self.mods.wasm.render_frame_pre(view, elapsed, 0.0);
            }
        }

        /// A mouse button of the window.
        pub(in crate::game) fn wasm_mouse(&mut self, button: MouseButton, pressed: bool) {
            let button = match button {
                MouseButton::Left => 0,
                MouseButton::Right => 1,
                MouseButton::Middle => 2,
                _ => return,
            };
            if self.mods.wasm.is_running() {
                let view = self.wasm_view();
                self.mods.wasm.mouse_button_pre(view, button, pressed);
            }
        }

        /// After the packs and the language reloaded.
        pub(in crate::game) fn wasm_resources_reloaded(&mut self) {
            if self.mods.wasm.is_running() {
                let view = self.wasm_view();
                self.mods.wasm.resources_reloaded(view);
            }
        }

        /// The wasm packages' rows on the Mods page: on until turned off, as native mods are,
        /// with a page of their settings' buttons.
        pub(in crate::game) fn wasm_mod_rows(&mut self) {
            let entries = self.mods.wasm.entries();
            self.settings
                .mods
                .installed
                .retain(|row| !entries.iter().any(|e| e.id == row.id));
            for e in entries {
                let buttons = self.wasm_settings_buttons(&e.id).unwrap_or_default();
                let problem = e
                    .problem
                    .clone()
                    .filter(|p| p != "Off")
                    .map(|p| format!("Stopped: {p}"));
                self.settings.mods.installed.push(Installed {
                    id: e.id.clone(),
                    title: format!("{} {}", e.name, e.version),
                    description: e.description.clone(),
                    problem,
                    dev: false,
                    has_settings: false,
                    native: Some(NativePage {
                        keys: Vec::new(),
                        buttons,
                    }),
                    details: ModDetails {
                        name: e.name,
                        version: e.version,
                        description: e.description,
                        authors: e.authors,
                        license: e.license,
                        minecraft: "26.3".into(),
                        homepage: e.homepage.unwrap_or_default(),
                        ..ModDetails::default()
                    },
                });
            }
        }

        /// A click on a wasm mod's settings button; false for another mod.
        pub(in crate::game) fn wasm_settings_click(&mut self, id: &str, button: usize) -> bool {
            if !self.mods.wasm.entries().iter().any(|e| e.id == id) {
                return false;
            }
            let view = self.wasm_view();
            self.mods.wasm.settings_click(id, button, view);
            true
        }

        /// A wasm mod's settings buttons; None for another mod.
        pub(in crate::game) fn wasm_settings_buttons(&mut self, id: &str) -> Option<Vec<String>> {
            if !self.mods.wasm.entries().iter().any(|e| e.id == id) {
                return None;
            }
            let view = self.wasm_view();
            Some(self.mods.wasm.settings_labels(id, view).unwrap_or_default())
        }
    }
}
