//! Mods' GeckoLib-style models (`render.spawn-model`): Bedrock geometry and animations from a
//! mod's `models/` folder, textured from an atlas model slot, drawn as entity geometry.
use super::Game;
use horizon_client::{
    mesher::Vertex,
    mobs::{Part, anim::Poses, keyframes::Animation},
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

type Key = (PathBuf, String);

#[derive(Default)]
pub(super) struct ModModels {
    geometry: HashMap<Key, Option<Arc<Part>>>,
    animations: HashMap<Key, Arc<HashMap<String, Animation>>>,
    /// Textures in the atlas's model slots, by file, with the frame they were last drawn.
    slots: Vec<Option<(Key, u64, [f32; 2])>>,
    /// Small textures (64 by 32 at most, a cape) eight to a slot: the cells of the last slots.
    cells: Vec<Option<(Key, u64, [f32; 2])>>,
    failed: HashMap<Key, ()>,
    /// When each playing animation (by serial) started, in seconds.
    started: HashMap<u64, f64>,
    frame: u64,
    /// Mods' icons in the icon slots, by slot.
    icons: Vec<Option<(Key, [f32; 4])>>,
    /// Downloaded skins' pixels, by atlas name, for `render.player-skin`.
    pub(super) downloaded: HashMap<String, Arc<[u8]>>,
    /// The skin `render.player-skin` last gave.
    skin: String,
    /// The atlas sprite of the cape a mod gives the local player (`render.set-player-cape`).
    pub(super) cape: Option<String>,
}

const MOD_CAPE: &str = "horizon/modcape";

/// Small textures to one 128 by 128 model slot: two columns of four 64 by 32 cells.
const CELLS_PER_SLOT: usize = 8;

const PLAYER_SKIN: &str = "player:skin";

fn hash(text: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut h);
    format!("json:{:016x}:{}", h.finish(), text.len())
}

fn read(path: &Path, file: &str) -> Option<Vec<u8>> {
    horizon_mods::Package::read_file(path, &format!("models/{file}"))
}

impl Game<'_> {
    /// The models running mods placed, as camera-relative entity geometry.
    #[cfg(feature = "mods")]
    pub(super) fn mod_model_geometry(&mut self, camera: glam::DVec3, out: &mut Vec<Vertex>) {
        self.sync_player_skin();
        self.sync_player_cape();
        let wanted: Vec<(PathBuf, horizon_mods::ModModel)> = self
            .mods
            .mods
            .models()
            .filter(|(_, _, m)| m.attach.is_none())
            .map(|(_, path, m)| (path.to_owned(), m.clone()))
            .collect();
        let attached: Vec<u64> = self
            .mods
            .mods
            .models()
            .filter_map(|(_, _, m)| m.attach.as_ref().and(m.animation.as_ref()).map(|a| a.3))
            .collect();
        self.mod_models.frame += 1;
        if wanted.is_empty() {
            self.mod_models.started.retain(|s, _| attached.contains(s));
            return;
        }
        let now = self.started.elapsed().as_secs_f64();
        let mut live = attached;
        for (path, m) in wanted {
            let Some((geometry, uv)) = self.mod_model_parts(&path, &m) else { continue };
            let mut poses = Poses::new(&geometry);
            self.pose_mod_model(&path, &m, now, &mut live, &mut poses);
            let feet = glam::DVec3::new(m.place.x, m.place.y, m.place.z);
            let rel = feet - camera;
            if rel.length_squared() > 192.0 * 192.0 {
                continue;
            }
            let light = self.world.light(feet.x.floor() as i32, (feet.y + 0.5).floor() as i32, feet.z.floor() as i32);
            let packed = (u32::from(light >> 4) * 16) | ((u32::from(light & 15) * 16) << 8);
            let scale = if m.place.scale.is_finite() { m.place.scale.clamp(0.01, 16.0) } else { 1.0 };
            // LivingEntityRenderer's model space: turned by the yaw, flipped, 1.501 down.
            let base = glam::Mat4::from_translation(rel.as_vec3())
                * glam::Mat4::from_rotation_y((180.0 - m.place.yaw).to_radians())
                * glam::Mat4::from_rotation_x(m.place.pitch.to_radians())
                * glam::Mat4::from_scale(glam::Vec3::splat(scale))
                * glam::Mat4::from_scale(glam::Vec3::new(-1.0, -1.0, 1.0))
                * glam::Mat4::from_translation(glam::Vec3::new(0.0, -1.501, 0.0));
            horizon_client::mobs::render::draw(&geometry, base, &poses, uv, 0xFFFF_FFFF, packed, 0, out);
        }
        self.mod_models.started.retain(|s, _| live.contains(s));
    }
    #[cfg(feature = "mods")]
    fn pose_mod_model(&mut self, path: &Path, m: &horizon_mods::ModModel, now: f64, live: &mut Vec<u64>, poses: &mut Poses) {
            if let Some((file, name, looping, serial)) = &m.animation {
                live.push(*serial);
                let akey = (path.to_owned(), file.clone());
                let set = self
                    .mod_models
                    .animations
                    .entry(akey)
                    .or_insert_with(|| {
                        let json = read(path, file).and_then(|b| serde_json::from_slice(&b).ok());
                        Arc::new(json.map(|j| horizon_client::mobs::bedrock::animations(&j)).unwrap_or_default())
                    })
                    .clone();
                if let Some(anim) = set.get(name) {
                    let start = *self.mod_models.started.entry(*serial).or_insert(now);
                    let elapsed = now - start;
                    let mut a = anim.clone();
                    a.looping = *looping;
                    let ms = if *looping { elapsed } else { elapsed.min(f64::from(anim.length)) };
                    a.apply((ms * 1000.0) as i64, 1.0, poses);
                }
            }
            for (bone, (rot, visible)) in &m.bones {
                let p = poses.get(bone);
                p[3] -= rot[0].to_radians();
                p[4] -= rot[1].to_radians();
                p[5] += rot[2].to_radians();
                poses.visible(bone, *visible);
            }
    }
    /// A mod model's geometry and texture rectangle.
    #[cfg(feature = "mods")]
    fn mod_model_parts(&mut self, path: &Path, m: &horizon_mods::ModModel) -> Option<(Arc<Part>, [f32; 4])> {
        let name = match &m.json {
            Some(text) => hash(text),
            None => m.geometry.clone(),
        };
        if self.mod_models.geometry.len() > 256 {
            self.mod_models.geometry.clear();
        }
        let geometry = self
            .mod_models
            .geometry
            .entry((path.to_owned(), name))
            .or_insert_with(|| {
                let json = match &m.json {
                    Some(text) => serde_json::from_str(text).ok()?,
                    None => serde_json::from_slice(&read(path, &m.geometry)?).ok()?,
                };
                horizon_client::mobs::bedrock::geometry(&json).ok().map(Arc::new)
            })
            .clone()?;
        let uv = if m.texture == PLAYER_SKIN {
            let skin = horizon_client::mobs::render::body(self.player_model.as_ref()?).1;
            let r = self.gui.atlas.sprites.get(&skin)?;
            [r.u0, r.v0, r.u1, r.v1]
        } else {
            self.model_texture(path, &format!("models/{}", m.texture))?
        };
        Some((geometry, uv))
    }
    /// The models mods hung on the local player's parts, drawn with its model in third person.
    #[cfg(feature = "mods")]
    pub(super) fn mod_attached_geometry(&mut self, camera: glam::DVec3, alpha: f64, out: &mut Vec<Vertex>) {
        if !self.player_model_shown() || !self.own_body_visible() {
            return;
        }
        let wanted: Vec<(PathBuf, horizon_mods::ModModel)> = self
            .mods
            .mods
            .models()
            .filter(|(_, _, m)| m.attach.is_some())
            .map(|(_, path, m)| (path.to_owned(), m.clone()))
            .collect();
        if wanted.is_empty() {
            return;
        }
        let now = self.started.elapsed().as_secs_f64();
        let mut live: Vec<u64> = self.mod_models.started.keys().copied().collect();
        for (path, m) in wanted {
            let Some((geometry, uv)) = self.mod_model_parts(&path, &m) else { continue };
            let Some(player) = self.player_model.as_ref() else { return };
            let part = m.attach.as_deref().unwrap_or_default();
            let Some(base) = horizon_client::mobs::render::part_transform(player, alpha as f32, camera, self.res.mob_data(), part) else {
                continue;
            };
            let light = self.world.light(
                player.position.x.floor() as i32,
                (player.position.y + 1.62).floor() as i32,
                player.position.z.floor() as i32,
            );
            let packed = (u32::from(light >> 4) * 16) | ((u32::from(light & 15) * 16) << 8);
            let mut poses = Poses::new(&geometry);
            self.pose_mod_model(&path, &m, now, &mut live, &mut poses);
            let scale = if m.place.scale.is_finite() { m.place.scale.clamp(0.01, 16.0) } else { 1.0 };
            let base = base * glam::Mat4::from_scale(glam::Vec3::splat(scale));
            horizon_client::mobs::render::draw(&geometry, base, &poses, uv, 0xFFFF_FFFF, packed, 0, out);
        }
    }
    #[cfg(not(feature = "mods"))]
    pub(super) fn mod_attached_geometry(&mut self, _camera: glam::DVec3, _alpha: f64, _out: &mut Vec<Vertex>) {}
    /// The cape a mod gives the local player: uploaded, kept in the atlas and worn.
    #[cfg(feature = "mods")]
    pub(super) fn sync_player_cape(&mut self) {
        let wanted = self.mods.mods.player_cape().map(|(p, f)| (p.to_owned(), f.to_owned()));
        let sprite = wanted.and_then(|(path, file)| {
            let [u0, v0, u1, v1] = self.model_texture(&path, &format!("models/{file}"))?;
            let mut r = self.gui.atlas.sprites.get("horizon/model0").copied()?;
            (r.u0, r.v0, r.u1, r.v1) = (u0, v0, u1, v1);
            self.gui.atlas.sprites.insert(MOD_CAPE.to_owned(), r);
            Some(MOD_CAPE.to_owned())
        });
        if sprite == self.mod_models.cape {
            if let (Some(s), Some(m)) = (&sprite, self.player_model.as_mut()) {
                m.cape = Some(s.clone());
            }
            return;
        }
        let was = self.mod_models.cape.take();
        if let Some(m) = self.player_model.as_mut() {
            match &sprite {
                Some(s) => m.cape = Some(s.clone()),
                None if was.is_some() && m.cape.as_deref() == Some(MOD_CAPE) => m.cape = None,
                None => {}
            }
        }
        self.mod_models.cape = sprite;
    }
    /// Gives mods the local player's skin (`render.player-skin`) when it changes.
    #[cfg(feature = "mods")]
    fn sync_player_skin(&mut self) {
        let Some(player) = self.player_model.as_ref() else { return };
        let slim = player.slim;
        let name = horizon_client::mobs::render::body(player).1;
        if name == self.mod_models.skin {
            if let Ok(mut s) = horizon_mods::PLAYER_SKIN.lock() {
                s.slim = slim;
            }
            return;
        }
        let pixels = match self.mod_models.downloaded.get(&name) {
            Some(p) => Some(p.clone()),
            None => horizon_client::hud::decode(&self.vanilla_root().join(format!("assets/minecraft/textures/{name}.png")))
                .ok()
                .filter(|(w, h, _)| *w == 64 && *h == 64)
                .map(|(_, _, p)| Arc::from(p)),
        };
        if let Ok(mut s) = horizon_mods::PLAYER_SKIN.lock() {
            s.slim = slim;
            s.pixels = pixels;
        }
        self.mod_models.skin = name;
    }
    /// The bits of `hidden_model_parts` for the parts mods hide (`render.hide-player-parts`).
    #[cfg(feature = "mods")]
    pub(super) fn mod_hidden_parts() -> u8 {
        let Ok(s) = horizon_mods::PLAYER_SKIN.lock() else { return 0 };
        s.hidden.iter().fold(0, |bits, p| {
            bits | match p.as_str() {
                "jacket" => 1 << 1,
                "left_sleeve" => 1 << 2,
                "right_sleeve" => 1 << 3,
                "left_pants" => 1 << 4,
                "right_pants" => 1 << 5,
                "hat" => 1 << 6,
                _ => 0,
            }
        })
    }
    #[cfg(not(feature = "mods"))]
    pub(super) fn mod_hidden_parts() -> u8 {
        0
    }
    #[cfg(not(feature = "mods"))]
    pub(super) fn mod_model_geometry(&mut self, _camera: glam::DVec3, _out: &mut Vec<Vertex>) {}

    /// The atlas rectangle of a mod's icon, in icon slot `slot` (32x32, scaled to fit).
    pub(super) fn mod_icon(&mut self, slot: usize, path: &Path, file: &str) -> Option<[f32; 4]> {
        if slot >= horizon_client::hud::MOD_ICON_SLOTS || file.is_empty() {
            return None;
        }
        let key = (path.to_owned(), file.to_owned());
        if self.mod_models.icons.len() < horizon_client::hud::MOD_ICON_SLOTS {
            self.mod_models.icons.resize(horizon_client::hud::MOD_ICON_SLOTS, None);
        }
        if let Some((k, r)) = &self.mod_models.icons[slot]
            && *k == key
        {
            return Some(*r);
        }
        let (iw, ih, pixels) = horizon_mods::Package::read_file(path, file)
            .and_then(|b| horizon_client::hud::decode_bytes(&b).ok())
            .filter(|(w, h, _)| *w > 0 && *h > 0 && *w <= 2048 && *h <= 2048)?;
        let mut rgba = vec![0u8; 32 * 32 * 4];
        for y in 0..32usize {
            for x in 0..32usize {
                let sx = (x as u32 * iw / 32).min(iw - 1) as usize;
                let sy = (y as u32 * ih / 32).min(ih - 1) as usize;
                let from = (sy * iw as usize + sx) * 4;
                rgba[(y * 32 + x) * 4..(y * 32 + x) * 4 + 4].copy_from_slice(&pixels[from..from + 4]);
            }
        }
        let r = self.gui.atlas.sprites.get(&format!("horizon/modicon{slot}")).copied()?;
        let (w, h) = (self.gui.atlas.width as f32, self.gui.atlas.height as f32);
        self.map_patches.push(horizon_client::font::Patch {
            x: (r.u0 * w).round() as u32,
            y: (r.v0 * h).round() as u32,
            width: 32,
            height: 32,
            rgba,
        });
        let rect = [r.u0, r.v0, r.u1, r.v1];
        self.mod_models.icons[slot] = Some((key, rect));
        Some(rect)
    }
    /// The icon of a mod that ships none: a purple block, as Mod Menu draws its
    /// placeholder.
    pub(super) fn mod_default_icon(&mut self, slot: usize) -> Option<[f32; 4]> {
        if slot >= horizon_client::hud::MOD_ICON_SLOTS {
            return None;
        }
        let key = (PathBuf::new(), "<default>".to_owned());
        if self.mod_models.icons.len() < horizon_client::hud::MOD_ICON_SLOTS {
            self.mod_models.icons.resize(horizon_client::hud::MOD_ICON_SLOTS, None);
        }
        if let Some((k, r)) = &self.mod_models.icons[slot]
            && *k == key
        {
            return Some(*r);
        }
        let mut rgba = vec![0u8; 32 * 32 * 4];
        for y in 0..32usize {
            for x in 0..32usize {
                let edge = x < 2 || y < 2 || x >= 30 || y >= 30;
                let top = (4..16).contains(&y) && x >= 4 + (16 - y) / 2 && x < 28 - (16 - y) / 2;
                let (r, g, b) = if edge {
                    (0xa0, 0x5c, 0xd0)
                } else if top {
                    (0x8a, 0x44, 0xb8)
                } else {
                    (0x57, 0x19, 0x7f)
                };
                let at = (y * 32 + x) * 4;
                rgba[at..at + 4].copy_from_slice(&[r, g, b, 255]);
            }
        }
        let r = self.gui.atlas.sprites.get(&format!("horizon/modicon{slot}")).copied()?;
        let (w, h) = (self.gui.atlas.width as f32, self.gui.atlas.height as f32);
        self.map_patches.push(horizon_client::font::Patch {
            x: (r.u0 * w).round() as u32,
            y: (r.v0 * h).round() as u32,
            width: 32,
            height: 32,
            rgba,
        });
        let rect = [r.u0, r.v0, r.u1, r.v1];
        self.mod_models.icons[slot] = Some((key, rect));
        Some(rect)
    }
    /// The atlas rectangle of a PNG in a mod (`models/x.png`, its icon), uploading it to a
    /// model slot (the one drawn longest ago) the first time.
    pub(super) fn model_texture(&mut self, path: &Path, file: &str) -> Option<[f32; 4]> {
        let key = (path.to_owned(), file.to_owned());
        let frame = self.mod_models.frame;
        let cell_slots = horizon_client::hud::MODEL_CELL_SLOTS;
        let slots = horizon_client::hud::MODEL_SLOTS - cell_slots;
        let cells = cell_slots * CELLS_PER_SLOT;
        if self.mod_models.slots.len() < slots {
            self.mod_models.slots.resize(slots, None);
        }
        if self.mod_models.cells.len() < cells {
            self.mod_models.cells.resize(cells, None);
        }
        let rect = |game: &Self, i: usize| {
            game.gui.atlas.sprites.get(&format!("horizon/model{i}")).copied()
        };
        let cell_uv = |game: &Self, c: usize, size: [f32; 2]| {
            let r = rect(game, slots + c / CELLS_PER_SLOT)?;
            let k = c % CELLS_PER_SLOT;
            let (ox, oy) = ((k % 2) as f32 * 0.5, (k / 2) as f32 * 0.25);
            let (du, dv) = (r.u1 - r.u0, r.v1 - r.v0);
            let (u0, v0) = (r.u0 + du * ox, r.v0 + dv * oy);
            Some([u0, v0, u0 + du * size[0], v0 + dv * size[1]])
        };
        if let Some(c) = self.mod_models.cells.iter().position(|s| s.as_ref().is_some_and(|(k, ..)| *k == key)) {
            let size = self.mod_models.cells[c].as_ref().map_or([0.5, 0.25], |s| s.2);
            self.mod_models.cells[c] = Some((key, frame, size));
            return cell_uv(self, c, size);
        }
        if let Some(i) = self.mod_models.slots.iter().position(|s| s.as_ref().is_some_and(|(k, ..)| *k == key)) {
            let size = self.mod_models.slots[i].as_ref().map_or([1.0; 2], |s| s.2);
            self.mod_models.slots[i] = Some((key, frame, size));
            let r = rect(self, i)?;
            return Some([r.u0, r.v0, r.u0 + (r.u1 - r.u0) * size[0], r.v0 + (r.v1 - r.v0) * size[1]]);
        }
        if self.mod_models.failed.contains_key(&key) {
            return None;
        }
        let image = horizon_mods::Package::read_file(path, file)
            .and_then(|b| horizon_client::hud::decode_bytes(&b).ok())
            .filter(|(w, h, _)| (1..=128).contains(w) && (1..=128).contains(h));
        let Some((iw, ih, pixels)) = image else {
            self.mod_models.failed.insert(key, ());
            return None;
        };
        let (w, h) = (self.gui.atlas.width as f32, self.gui.atlas.height as f32);
        let size = [iw as f32 / 128.0, ih as f32 / 128.0];
        if iw <= 64 && ih <= 32 && cells > 0 {
            let c = (0..cells)
                .min_by_key(|c| self.mod_models.cells[*c].as_ref().map_or(0, |(_, f, _)| *f + 1))
                .unwrap_or(0);
            if self.mod_models.cells[c].as_ref().is_some_and(|(_, f, _)| *f == frame) {
                return None;
            }
            let r = rect(self, slots + c / CELLS_PER_SLOT)?;
            let k = c % CELLS_PER_SLOT;
            let mut rgba = vec![0u8; 64 * 32 * 4];
            for y in 0..ih as usize {
                let row = &pixels[y * iw as usize * 4..(y + 1) * iw as usize * 4];
                rgba[y * 64 * 4..y * 64 * 4 + row.len()].copy_from_slice(row);
            }
            self.map_patches.push(horizon_client::font::Patch {
                x: (r.u0 * w).round() as u32 + (k % 2) as u32 * 64,
                y: (r.v0 * h).round() as u32 + (k / 2) as u32 * 32,
                width: 64,
                height: 32,
                rgba,
            });
            self.mod_models.cells[c] = Some((key, frame, size));
            return cell_uv(self, c, size);
        }
        let i = (0..slots)
            .min_by_key(|i| self.mod_models.slots[*i].as_ref().map_or(0, |(_, f, _)| *f + 1))
            .unwrap_or(0);
        if self.mod_models.slots[i].as_ref().is_some_and(|(_, f, _)| *f == frame) {
            return None;
        }
        let r = rect(self, i)?;
        // The slot is 128x128: the texture fills its top-left corner.
        let mut rgba = vec![0u8; 128 * 128 * 4];
        for y in 0..ih as usize {
            let row = &pixels[y * iw as usize * 4..(y + 1) * iw as usize * 4];
            rgba[y * 128 * 4..y * 128 * 4 + row.len()].copy_from_slice(row);
        }
        self.map_patches.push(horizon_client::font::Patch {
            x: (r.u0 * w).round() as u32,
            y: (r.v0 * h).round() as u32,
            width: 128,
            height: 128,
            rgba,
        });
        self.mod_models.slots[i] = Some((key, frame, size));
        Some([r.u0, r.v0, r.u0 + (r.u1 - r.u0) * size[0], r.v0 + (r.v1 - r.v0) * size[1]])
    }
}
