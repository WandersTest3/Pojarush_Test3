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
    use pojarush_wasm_mods::{DrawCmd, View, WasmMods};

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
            self.mods.wasm = WasmMods::start(&dir, &game_dir, &off);
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
            }
        }

        /// After the native mods' boxes: each wasm mod's, below what the corners hold.
        pub(in crate::game) fn wasm_hud(&mut self, batch: &mut HudBatch, corners: &mut Corners) {
            if !self.mods.wasm.is_running() {
                return;
            }
            let view = self.wasm_view();
            let (w, h, used) = corners.parts();
            let (cmds, used) = self.mods.wasm.render_gui_post(view, (w, h), used);
            *corners = Corners::from_parts(w, h, used);
            let atlas = &self.gui.atlas;
            for cmd in cmds {
                match cmd {
                    DrawCmd::Fill { rect: [x, y, w, h], argb } => {
                        let [a, r, g, b] = argb.to_be_bytes();
                        batch.fill(atlas, x, y, w, h, rgba(r, g, b, a));
                    }
                    DrawCmd::Text { text, at: [x, y], argb, shadow } => {
                        let [a, r, g, b] = argb.to_be_bytes();
                        let a = if a == 0 { 0xFF } else { a };
                        batch.text_styled(atlas, &Text::literal(text), x, y, rgba(r, g, b, a), shadow);
                    }
                }
            }
        }

        /// Once a frame.
        pub(in crate::game) fn wasm_frame(&mut self, elapsed: Duration) {
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
