//! Capability `input` (docs/mods.md): the use button on items a mod claims,
//! the mod's own keys and the mouse wheel; and `render`'s zoom. With no mod
//! claiming items, keys or the wheel, each hook here returns after one
//! check.
use super::Game;
use winit::{event::MouseButton, keyboard::KeyCode};
#[cfg(feature = "mods")]
use {super::Screen, horizon_mods::HeldItem};

/// A claimed item's use while the button is down.
#[cfg(feature = "mods")]
pub(super) struct ModUse {
    item: HeldItem,
    /// `Game::ticks` when it started.
    since: u64,
}

#[cfg(not(feature = "mods"))]
impl Game<'_> {
    pub(super) fn mod_hud_mouse(&mut self, _pressed: bool) -> bool {
        false
    }
    pub(super) fn mod_hud_mouse_move(&mut self) {}
    pub(super) fn sandbox_mod_use(&mut self, _button: MouseButton, _pressed: bool) -> bool {
        false
    }
    pub(super) fn mod_key(&mut self, _key: KeyCode, _pressed: bool, _repeat: bool) {}
    pub(super) fn mod_mouse_key(&mut self, _button: MouseButton, _pressed: bool) {}
    pub(super) fn mod_wheel(&mut self, _steps: f32) -> bool {
        false
    }
    pub(super) fn mod_zoom(&self) -> Option<ModZoom> {
        None
    }
    pub(super) fn mod_minimap(&self) -> Option<ModMinimap> {
        None
    }
    pub(super) fn mod_far_terrain(&self) -> Option<ModFarTerrain> {
        None
    }
}

/// A mod's zoom (`render.set-zoom`), clamped by the runtime.
#[derive(Clone, Copy)]
pub(super) struct ModZoom {
    pub(super) factor: f32,
    pub(super) smooth: bool,
    pub(super) turn: f32,
}

/// A mod's minimap (`render.set-minimap`), clamped by the runtime.
#[derive(Clone, Copy)]
pub(super) struct ModMinimap {
    pub(super) corner: horizon_client::extras::Corner,
    pub(super) size: f32,
    pub(super) blocks_per_pixel: f32,
    pub(super) rotate: bool,
    pub(super) round: bool,
}

/// A mod's far terrain (`render.set-far-terrain`), clamped by the runtime.
#[derive(Clone, Copy)]
pub(super) struct ModFarTerrain {
    pub(super) distance: u32,
    pub(super) reduced: bool,
}

#[cfg(feature = "mods")]
impl Game<'_> {
    /// The main hand's stack as `input` shows it; none when it is empty.
    /// The player's stacks for `input.inventory`: slot, item, count.
    pub(super) fn mod_inventory(&self) -> std::sync::Arc<[(u32, String, u32)]> {
        let stacks = self.inventory.iter().take(36).enumerate().map(|(i, s)| (i as u32, s.as_ref()));
        let armor = self.armor.iter().enumerate().map(|(i, s)| (36 + i as u32, s.as_ref()));
        stacks
            .chain(armor)
            .chain(std::iter::once((40, self.offhand.as_ref())))
            .filter_map(|(i, s)| s.map(|s| (i, s.item.to_string(), s.count)))
            .collect()
    }
    /// `input.swap-offhand`: inventory slot `slot` (0 to 35) with the off hand, as F over the
    /// slot in the inventory screen (a SWAP click with button 40 on the player's own menu).
    /// Skipped while another container is open or the cursor holds a stack.
    pub(super) fn mod_swap_offhand(&mut self, slot: usize) {
        if slot >= 36 || slot >= self.inventory.len() || self.carried.is_some() || self.player.is_none() {
            log::debug!("input.swap-offhand: slot {slot} skipped (no player, or a stack on the cursor)");
            return;
        }
        let (a, b) = (self.inventory[slot].clone(), self.offhand.clone());
        if a.is_none() && b.is_none() {
            return;
        }
        if self.remote.is_some() && self.creative {
            // Creative: the client's slots are the truth, sent as `SetCreativeModeSlot`.
            self.inventory[slot] = b;
            self.offhand = a;
            self.send_creative_slots();
        } else if self.remote.is_some() {
            if self.remote_menu().0 != 0 {
                log::debug!("input.swap-offhand: skipped, a container is open");
                return;
            }
            // InventoryMenu: the hotbar is slots 36 to 44, the rest 9 to 35, the off hand 45.
            let menu_slot = if slot < 9 { 36 + slot } else { slot } as i16;
            self.inventory[slot] = b.clone();
            self.offhand = a.clone();
            self.send_clicks(
                0,
                vec![(menu_slot, 40, horizon_client::menu::Input::Swap, vec![(menu_slot, b), (45, a)], None)],
            );
        } else {
            self.inventory[slot] = b;
            self.offhand = a;
        }
    }
    /// `input.current-screen`: 0 playing, 1 the player's own inventory, 2 another screen.
    pub(super) fn mod_screen_kind(&self) -> u8 {
        if self.title.is_some() || self.mod_screen_open() {
            2
        } else if self.screen == Screen::Playing {
            0
        } else if self.screen == Screen::Inventory && self.station.is_none() && self.container.is_none() && self.remote_menu_id() == 0 {
            1
        } else {
            2
        }
    }
    /// `input.open-inventory`: the inventory key's path, while playing in survival or adventure.
    pub(super) fn mod_open_inventory(&mut self) {
        if self.player.is_none() || self.creative || self.spectator() || self.mod_screen_kind() != 0 || self.riding_mount_with_menu() {
            log::debug!("input.open-inventory: skipped");
            return;
        }
        if let Err(e) = self.key_pressed(KeyCode::KeyE) {
            log::warn!("input.open-inventory: {e}");
        }
    }
    /// `input.select-hotbar`: the number key's path, while playing and not a spectator.
    pub(super) fn mod_select_hotbar(&mut self, slot: usize) {
        if slot >= 9 || self.player.is_none() || self.spectator() || self.mod_screen_kind() != 0 || self.screen != Screen::Playing {
            log::debug!("input.select-hotbar: slot {slot} skipped");
            return;
        }
        self.selected = slot;
    }
    /// `input.swap-hands`: the swap-hands key's path, while playing and not a spectator.
    pub(super) fn mod_swap_hands(&mut self) {
        if self.player.is_none() || self.spectator() || self.mod_screen_kind() != 0 || self.screen != Screen::Playing {
            log::debug!("input.swap-hands: skipped");
            return;
        }
        if let Err(e) = self.key_pressed(KeyCode::KeyF) {
            log::warn!("input.swap-hands: {e}");
        }
    }
    /// `input.close-inventory`: closes the player's own inventory as Esc does.
    pub(super) fn mod_close_inventory(&mut self) {
        if self.mod_screen_kind() != 1 {
            log::debug!("input.close-inventory: skipped");
            return;
        }
        if let Err(e) = self.key_pressed(KeyCode::Escape) {
            log::warn!("input.close-inventory: {e}");
        }
    }
    pub(super) fn mod_held(&self) -> Option<HeldItem> {
        let stack = self.inventory.get(self.selected)?.as_ref()?;
        Some(HeldItem {
            id: stack.item.to_string(),
            count: stack.count,
            slot: self.selected as u32,
        })
    }
    /// The use button while playing: true when a mod claimed the held item,
    /// so the game does nothing else with it (no placing, eating, swing).
    pub(super) fn sandbox_mod_use(&mut self, button: MouseButton, pressed: bool) -> bool {
        if button != MouseButton::Right || !self.mods.mods.claims_any() {
            return false;
        }
        if !pressed {
            return self.stop_mod_use();
        }
        let Some(item) = self.mod_held().filter(|h| self.mods.mods.claims(&h.id)) else {
            return false;
        };
        self.mods.mods.use_start(&item);
        self.mods.using = Some(ModUse {
            item,
            since: self.ticks,
        });
        true
    }
    /// Ends a claimed use: the button came up, the player switched slots,
    /// or a screen opened. False when none was going.
    pub(super) fn stop_mod_use(&mut self) -> bool {
        let Some(u) = self.mods.using.take() else {
            return false;
        };
        let ticks = self.ticks.saturating_sub(u.since).min(u64::from(u32::MAX)) as u32;
        self.mods.mods.use_stop(&u.item, ticks);
        true
    }
    /// Once a tick: a claimed use stops when its item leaves the hand or a
    /// screen opens, as vanilla's `releaseUsingItem`.
    pub(super) fn tick_sandbox_mod_use(&mut self) {
        let Some(u) = &self.mods.using else {
            return;
        };
        let still = self
            .mod_held()
            .is_some_and(|h| h.id == u.item.id && h.slot == u.item.slot);
        if !still || self.screen != Screen::Playing {
            self.stop_mod_use();
        }
    }
    /// A key went down or up: a mod's registered key while playing (no
    /// screen) is its `key-down` / `key-up`. Vanilla's mapping on the same
    /// key still runs.
    pub(super) fn mod_key(&mut self, key: KeyCode, pressed: bool, repeat: bool) {
        if repeat || !self.mods.mods.has_keys() {
            return;
        }
        // Over an item on a container screen, the mods' keys are `browser`
        // events (a recipe viewer's R and U).
        if pressed
            && self.screen == Screen::Inventory
            && self.title.is_none()
            && !self.mod_browser_typing()
            && let Some(name) = horizon_client::keybinds::key_name(key)
            && let Some(stack) = self.mod_browser_pointed()
        {
            let stack = super::mod_panel::browser_stack(stack);
            self.mods.mods.browser_key(name, &stack);
            return;
        }
        if pressed && (self.screen != Screen::Playing || self.title.is_some()) {
            return;
        }
        if let Some(name) = horizon_client::keybinds::key_name(key) {
            self.mods.mods.key(name, pressed);
        }
    }
    /// A mouse button while playing: a mod's key bound to it
    /// (`key.mouse.4`...), as `mod_key`.
    pub(super) fn mod_mouse_key(&mut self, button: MouseButton, pressed: bool) {
        if self.mods.mods.has_keys() {
            let name = horizon_client::keybinds::mouse_name(button);
            self.mods.mods.key(&name, pressed);
        }
    }
    /// Wheel steps while playing: true when a mod holds the wheel, so the
    /// hotbar doesn't move.
    pub(super) fn mod_wheel(&mut self, steps: f32) -> bool {
        self.mods.mods.wheel(steps)
    }
    /// Playing, with nothing over the world: mods' keys and zoom apply.
    fn mod_playing(&self) -> bool {
        self.screen == Screen::Playing
            && self.title.is_none()
            && self.sign_edit.is_none()
            && self.book.is_none()
    }
    /// The left button over a mod's draggable HUD area (`render.set-hud-drag`)
    /// while the cursor is free, in any screen: the drag starts and the click
    /// goes no further. Release ends it. True when the event is taken.
    pub(super) fn mod_hud_mouse(&mut self, pressed: bool) -> bool {
        let (x, y) = self.cursor;
        if !pressed {
            return match self.mods.hud_mouse.take() {
                Some((i, lx, ly)) => {
                    self.mods.mods.push_hud_drag(i, x - lx, y - ly, true);
                    true
                }
                None => false,
            };
        }
        if self.captured || !self.mods.mods.is_running() {
            return false;
        }
        let hit = self.mods.mods.hud_drag_areas().into_iter().rev().find(|(_, r)| {
            x >= r[0] - 2.0 && x <= r[0] + r[2] + 2.0 && y >= r[1] - 2.0 && y <= r[1] + r[3] + 2.0
        });
        let Some((i, _)) = hit else {
            return false;
        };
        self.mods.hud_mouse = Some((i, x, y));
        true
    }
    /// The cursor moved: a held HUD area follows it.
    pub(super) fn mod_hud_mouse_move(&mut self) {
        let (x, y) = self.cursor;
        if let Some((i, lx, ly)) = self.mods.hud_mouse {
            self.mods.mods.push_hud_drag(i, x - lx, y - ly, false);
            self.mods.hud_mouse = Some((i, x, y));
        }
    }
    /// Once a frame, before the mods run: a screen that opened lets their
    /// held keys up.
    pub(super) fn mod_input_frame(&mut self) {
        #[cfg(target_os = "android")]
        {
            crate::android::set_hud_drag_areas(self.mods.mods.hud_drag_areas());
            for (i, dx, dy, done) in crate::android::take_hud_drag_moves() {
                self.mods.mods.push_hud_drag(i, dx, dy, done);
            }
        }
        let playing = self.mod_playing();
        if self.mods.playing && !playing {
            self.mods.mods.release_keys();
        }
        self.mods.playing = playing;
    }
    /// The strongest zoom a mod asks for, while playing; the spyglass's
    /// own view wins.
    pub(super) fn mod_zoom(&self) -> Option<ModZoom> {
        if !self.mod_playing() || self.scoping() {
            return None;
        }
        let z = self.mods.mods.zoom()?;
        Some(ModZoom {
            factor: z.factor,
            smooth: z.smooth,
            turn: z.turn,
        })
    }
    /// The minimap the first mod showing one asks for.
    pub(super) fn mod_minimap(&self) -> Option<ModMinimap> {
        use horizon_client::extras::Corner;
        use horizon_mods::RenderCorner;
        let m = self.mods.mods.minimap()?;
        Some(ModMinimap {
            corner: match m.corner {
                RenderCorner::TopLeft => Corner::TopLeft,
                RenderCorner::TopRight => Corner::TopRight,
                RenderCorner::BottomLeft => Corner::BottomLeft,
                RenderCorner::BottomRight => Corner::BottomRight,
            },
            size: m.size,
            blocks_per_pixel: m.blocks_per_pixel,
            rotate: m.rotate,
            round: m.round,
        })
    }
    /// The far terrain the first mod drawing one asks for.
    pub(super) fn mod_far_terrain(&self) -> Option<ModFarTerrain> {
        let f = self.mods.mods.far_terrain()?;
        Some(ModFarTerrain {
            distance: f.distance,
            reduced: f.reduced,
        })
    }
}
