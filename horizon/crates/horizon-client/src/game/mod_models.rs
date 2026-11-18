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
    failed: HashMap<Key, ()>,
    /// When each playing animation (by serial) started, in seconds.
    started: HashMap<u64, f64>,
    frame: u64,
}

fn read(path: &Path, file: &str) -> Option<Vec<u8>> {
    horizon_mods::Package::read_file(path, &format!("models/{file}"))
}

impl Game<'_> {
    /// The models running mods placed, as camera-relative entity geometry.
    #[cfg(feature = "mods")]
    pub(super) fn mod_model_geometry(&mut self, camera: glam::DVec3, out: &mut Vec<Vertex>) {
        let wanted: Vec<(PathBuf, horizon_mods::ModModel)> = self
            .mods
            .mods
            .models()
            .map(|(_, path, m)| (path.to_owned(), m.clone()))
            .collect();
        self.mod_models.frame += 1;
        if wanted.is_empty() {
            self.mod_models.started.clear();
            return;
        }
        let now = self.started.elapsed().as_secs_f64();
        let mut live = Vec::new();
        for (path, m) in wanted {
            let gkey = (path.clone(), m.geometry.clone());
            let geometry = self
                .mod_models
                .geometry
                .entry(gkey)
                .or_insert_with(|| {
                    let bytes = read(&path, &m.geometry)?;
                    let json = serde_json::from_slice(&bytes).ok()?;
                    horizon_client::mobs::bedrock::geometry(&json).ok().map(Arc::new)
                })
                .clone();
            let Some(geometry) = geometry else { continue };
            let Some(uv) = self.model_texture(&path, &m.texture) else { continue };
            let models = &mut self.mod_models;
            let mut poses = Poses::new(&geometry);
            if let Some((file, name, looping, serial)) = &m.animation {
                live.push(*serial);
                let akey = (path.clone(), file.clone());
                let set = models
                    .animations
                    .entry(akey)
                    .or_insert_with(|| {
                        let json = read(&path, file).and_then(|b| serde_json::from_slice(&b).ok());
                        Arc::new(json.map(|j| horizon_client::mobs::bedrock::animations(&j)).unwrap_or_default())
                    })
                    .clone();
                if let Some(anim) = set.get(name) {
                    let start = *models.started.entry(*serial).or_insert(now);
                    let elapsed = now - start;
                    let mut a = anim.clone();
                    a.looping = *looping;
                    let ms = if *looping { elapsed } else { elapsed.min(f64::from(anim.length)) };
                    a.apply((ms * 1000.0) as i64, 1.0, &mut poses);
                }
            }
            for (bone, (rot, visible)) in &m.bones {
                let p = poses.get(bone);
                p[3] -= rot[0].to_radians();
                p[4] -= rot[1].to_radians();
                p[5] += rot[2].to_radians();
                poses.visible(bone, *visible);
            }
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
    #[cfg(not(feature = "mods"))]
    pub(super) fn mod_model_geometry(&mut self, _camera: glam::DVec3, _out: &mut Vec<Vertex>) {}

    /// The atlas rectangle of a mod's model texture, uploading it to a model slot (the one
    /// drawn longest ago) the first time.
    fn model_texture(&mut self, path: &Path, file: &str) -> Option<[f32; 4]> {
        let key = (path.to_owned(), file.to_owned());
        let frame = self.mod_models.frame;
        let slots = horizon_client::hud::MODEL_SLOTS;
        if self.mod_models.slots.len() < slots {
            self.mod_models.slots.resize(slots, None);
        }
        let rect = |game: &Self, i: usize| {
            game.gui.atlas.sprites.get(&format!("horizon/model{i}")).copied()
        };
        if let Some(i) = self.mod_models.slots.iter().position(|s| s.as_ref().is_some_and(|(k, ..)| *k == key)) {
            let size = self.mod_models.slots[i].as_ref().map_or([1.0; 2], |s| s.2);
            self.mod_models.slots[i] = Some((key, frame, size));
            let r = rect(self, i)?;
            return Some([r.u0, r.v0, r.u0 + (r.u1 - r.u0) * size[0], r.v0 + (r.v1 - r.v0) * size[1]]);
        }
        if self.mod_models.failed.contains_key(&key) {
            return None;
        }
        let image = read(path, file)
            .and_then(|b| horizon_client::hud::decode_bytes(&b).ok())
            .filter(|(w, h, _)| (1..=128).contains(w) && (1..=128).contains(h));
        let Some((iw, ih, pixels)) = image else {
            self.mod_models.failed.insert(key, ());
            return None;
        };
        let i = (0..slots)
            .min_by_key(|i| self.mod_models.slots[*i].as_ref().map_or(0, |(_, f, _)| *f + 1))
            .unwrap_or(0);
        if self.mod_models.slots[i].as_ref().is_some_and(|(_, f, _)| *f == frame) {
            return None;
        }
        let r = rect(self, i)?;
        let (w, h) = (self.gui.atlas.width as f32, self.gui.atlas.height as f32);
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
        let size = [iw as f32 / 128.0, ih as f32 / 128.0];
        self.mod_models.slots[i] = Some((key, frame, size));
        Some([r.u0, r.v0, r.u0 + (r.u1 - r.u0) * size[0], r.v0 + (r.v1 - r.v0) * size[1]])
    }
}
