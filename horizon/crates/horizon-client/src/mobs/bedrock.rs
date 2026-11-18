//! GeckoLib's files for mods' models: Bedrock geometry (`.geo.json`) as a model part tree in
//! Java's model space, and Bedrock animations (`.animation.json`) as keyframe animations.
//!
//! Bedrock space is Java's with x mirrored and y up from the feet: a point (x, y, z) is
//! (-x, 24 - y, z) in a Java model, and a rotation (x, y, z) is (-x, -y, z), as Blockbench
//! converts between the two.
use super::{
    Part,
    keyframes::{Animation, Channel, Keyframe, Target},
};
use serde_json::Value;
use std::collections::HashMap;

fn v3(v: &Value, default: f32) -> [f32; 3] {
    let a = |i: usize| v.get(i).and_then(Value::as_f64).map_or(default, |x| x as f32);
    [a(0), a(1), a(2)]
}

fn java_point(p: [f32; 3]) -> [f32; 3] {
    [-p[0], 24.0 - p[1], p[2]]
}

fn java_rotation(r: [f32; 3]) -> [f32; 3] {
    [-r[0].to_radians(), -r[1].to_radians(), r[2].to_radians()]
}

/// The first geometry of a `.geo.json` (format 1.12+) as a part tree named after its bones.
pub fn geometry(json: &Value) -> Result<Part, String> {
    let geo = json["minecraft:geometry"]
        .get(0)
        .ok_or("no minecraft:geometry")?;
    let desc = &geo["description"];
    let tex = [
        desc["texture_width"].as_f64().unwrap_or(64.0) as f32,
        desc["texture_height"].as_f64().unwrap_or(64.0) as f32,
    ];
    let bones = geo["bones"].as_array().ok_or("no bones")?;
    let mut parts: Vec<(Option<String>, [f32; 3], Part)> = Vec::new();
    for bone in bones {
        let name = bone["name"].as_str().ok_or("a bone without a name")?.to_owned();
        let pivot = java_point(v3(&bone["pivot"], 0.0));
        let rot = java_rotation(v3(&bone["rotation"], 0.0));
        let mirror = bone["mirror"].as_bool().unwrap_or(false);
        let mut part = Part {
            name: name.clone(),
            pose: [pivot[0], pivot[1], pivot[2], rot[0], rot[1], rot[2], 1.0, 1.0, 1.0],
            polygons: Vec::new(),
            children: Vec::new(),
        };
        for (i, c) in bone["cubes"].as_array().into_iter().flatten().enumerate() {
            let origin = v3(&c["origin"], 0.0);
            let size = v3(&c["size"], 0.0);
            let inflate = c["inflate"].as_f64().unwrap_or(0.0) as f32;
            let mirror = c["mirror"].as_bool().unwrap_or(mirror);
            let min = java_point([origin[0] + size[0], origin[1] + size[1], origin[2]]);
            let (offset, faces) = uv(&c["uv"]);
            let rotated = c["rotation"].is_array();
            let base = if rotated { java_point(v3(&c["pivot"], 0.0)) } else { pivot };
            let from = [min[0] - base[0], min[1] - base[1], min[2] - base[2]];
            let polygons = super::cem::cube_polygons(offset, from, size, [inflate; 3], mirror, tex, &faces);
            if rotated {
                let r = java_rotation(v3(&c["rotation"], 0.0));
                part.children.push(Part {
                    name: format!("{name}#cube{i}"),
                    pose: [base[0] - pivot[0], base[1] - pivot[1], base[2] - pivot[2], r[0], r[1], r[2], 1.0, 1.0, 1.0],
                    polygons,
                    children: Vec::new(),
                });
            } else {
                part.polygons.extend(polygons);
            }
        }
        parts.push((bone["parent"].as_str().map(str::to_owned), pivot, part));
    }
    // Children's pivots are relative to their parent's; attach leaves first.
    let pivots: HashMap<String, [f32; 3]> =
        parts.iter().map(|(_, p, part)| (part.name.clone(), *p)).collect();
    for (parent, _, part) in &mut parts {
        if let Some(pp) = parent.as_ref().and_then(|p| pivots.get(p)) {
            part.pose[0] -= pp[0];
            part.pose[1] -= pp[1];
            part.pose[2] -= pp[2];
        }
    }
    let mut root = Part {
        name: "root".into(),
        pose: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
        polygons: Vec::new(),
        children: Vec::new(),
    };
    let mut pending: Vec<(Option<String>, Part)> = parts.into_iter().map(|(p, _, part)| (p, part)).collect();
    while !pending.is_empty() {
        let names: Vec<String> = pending.iter().map(|(_, p)| p.name.clone()).collect();
        let leaf = pending
            .iter()
            .position(|(_, p)| !pending.iter().any(|(parent, _)| parent.as_deref() == Some(&p.name)))
            .ok_or("bones form a cycle")?;
        let (parent, part) = pending.remove(leaf);
        match parent.filter(|p| names.contains(p)) {
            Some(p) => {
                if let Some((_, owner)) = pending.iter_mut().find(|(_, x)| x.name == p) {
                    owner.children.push(part);
                }
            }
            None => root.children.push(part),
        }
    }
    Ok(root)
}

/// A cube's UV: a box offset, or per-face rectangles (`uv` + `uv_size`).
fn uv(v: &Value) -> ([f32; 2], [Option<[f32; 4]>; 6]) {
    if let Some(a) = v.as_array() {
        let f = |i: usize| a.get(i).and_then(Value::as_f64).unwrap_or(0.0) as f32;
        return ([f(0), f(1)], [None; 6]);
    }
    let face = |n: &str| {
        let f = &v[n];
        let uv = f["uv"].as_array()?;
        let size = f["uv_size"].as_array();
        let g = |a: &Vec<Value>, i: usize| a.get(i).and_then(Value::as_f64).unwrap_or(0.0) as f32;
        let (u, w) = (g(uv, 0), size.map_or(0.0, |s| g(s, 0)));
        let (vv, h) = (g(uv, 1), size.map_or(0.0, |s| g(s, 1)));
        Some([u, vv, u + w, vv + h])
    };
    ([0.0; 2], ["down", "up", "east", "north", "west", "south"].map(face))
}

fn number(v: &Value) -> f32 {
    v.as_f64()
        .map(|x| x as f32)
        .or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
        .unwrap_or(0.0)
}

fn vec3(v: &Value) -> [f32; 3] {
    match v {
        Value::Array(a) => std::array::from_fn(|i| a.get(i).map_or(0.0, number)),
        other => [number(other); 3],
    }
}

/// The animations of a `.animation.json`, by name. Molang expressions are not evaluated: a
/// value that is not a number counts as 0.
pub fn animations(json: &Value) -> HashMap<String, Animation> {
    let mut out = HashMap::new();
    for (name, a) in json["animations"].as_object().into_iter().flatten() {
        let looping = a["loop"].as_bool().unwrap_or(false) || a["loop"].as_str() == Some("true");
        let mut length = a["animation_length"].as_f64().unwrap_or(0.0) as f32;
        let mut bones = Vec::new();
        for (bone, channels) in a["bones"].as_object().into_iter().flatten() {
            let mut list = Vec::new();
            for (key, target) in [("rotation", Target::Rotation), ("position", Target::Position), ("scale", Target::Scale)] {
                let ch = &channels[key];
                if ch.is_null() {
                    continue;
                }
                let convert = |x: [f32; 3]| match target {
                    Target::Rotation => java_rotation(x),
                    Target::Position => [-x[0], -x[1], x[2]],
                    Target::Scale => [x[0] - 1.0, x[1] - 1.0, x[2] - 1.0],
                };
                let mut keyframes = Vec::new();
                match ch.as_object() {
                    Some(times) if !times.contains_key("vector") => {
                        for (t, k) in times {
                            let time: f32 = t.parse().unwrap_or(0.0);
                            let (pre, post, catmull) = match k {
                                Value::Object(o) => {
                                    let post = o.get("post").map_or([0.0; 3], vec3);
                                    let pre = o.get("pre").map_or(post, vec3);
                                    (pre, post, o.get("lerp_mode").and_then(Value::as_str) == Some("catmullrom"))
                                }
                                other => (vec3(other), vec3(other), false),
                            };
                            length = length.max(time);
                            keyframes.push(Keyframe { time, pre: convert(pre), post: convert(post), catmull });
                        }
                    }
                    Some(o) => {
                        let x = convert(vec3(&o["vector"]));
                        keyframes.push(Keyframe { time: 0.0, pre: x, post: x, catmull: false });
                    }
                    None => {
                        let x = convert(vec3(ch));
                        keyframes.push(Keyframe { time: 0.0, pre: x, post: x, catmull: false });
                    }
                }
                keyframes.sort_by(|a, b| a.time.total_cmp(&b.time));
                list.push(Channel { target, keyframes });
            }
            bones.push((bone.clone(), list));
        }
        out.insert(
            name.clone(),
            Animation { length: length.max(0.05), looping, bones },
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bones_nest_with_relative_pivots() {
        let geo: Value = serde_json::from_str(
            r#"{"format_version":"1.12.0","minecraft:geometry":[{"description":{"texture_width":64,"texture_height":64},
               "bones":[{"name":"body","pivot":[0,24,0],"cubes":[{"origin":[-4,12,-2],"size":[8,12,4],"uv":[16,16]}]},
                        {"name":"head","parent":"body","pivot":[0,24,0],"rotation":[10,0,0],
                         "cubes":[{"origin":[-4,24,-4],"size":[8,8,8],"uv":[0,0]}]}]}]}"#,
        )
        .unwrap();
        let root = geometry(&geo).unwrap();
        let body = &root.children[0];
        assert_eq!(body.name, "body");
        assert_eq!(&body.pose[..3], &[0.0, 0.0, 0.0]);
        let head = &body.children[0];
        assert_eq!(&head.pose[..3], &[0.0, 0.0, 0.0]);
        assert!((head.pose[3] + 10f32.to_radians()).abs() < 1e-6);
        let ys: Vec<f32> = head.polygons.iter().flat_map(|p| p.vertices.map(|v| v[1])).collect();
        let (lo, hi) = ys.iter().fold((f32::MAX, f32::MIN), |(a, b), y| (a.min(*y), b.max(*y)));
        assert_eq!((lo, hi), (-8.0, 0.0));
        let ys: Vec<f32> = body.polygons.iter().flat_map(|p| p.vertices.map(|v| v[1])).collect();
        let (lo, hi) = ys.iter().fold((f32::MAX, f32::MIN), |(a, b), y| (a.min(*y), b.max(*y)));
        assert_eq!((lo, hi), (0.0, 12.0));
    }

    #[test]
    fn animations_become_keyframes() {
        let anim: Value = serde_json::from_str(
            r#"{"format_version":"1.8.0","animations":{"walk":{"loop":true,"animation_length":1.0,
               "bones":{"head":{"rotation":{"0.0":[0,0,0],"0.5":{"post":[30,0,0],"lerp_mode":"catmullrom"},"1.0":[0,0,0]},
                                 "position":[0,2,0]}}}}}"#,
        )
        .unwrap();
        let all = animations(&anim);
        let walk = &all["walk"];
        assert!(walk.looping);
        assert_eq!(walk.length, 1.0);
        let (bone, channels) = &walk.bones[0];
        assert_eq!(bone, "head");
        let rot = channels.iter().find(|c| c.target == Target::Rotation).unwrap();
        assert_eq!(rot.keyframes.len(), 3);
        assert!((rot.keyframes[1].post[0] + 30f32.to_radians()).abs() < 1e-6);
        assert!(rot.keyframes[1].catmull);
        let pos = channels.iter().find(|c| c.target == Target::Position).unwrap();
        assert_eq!(pos.keyframes[0].post, [0.0, -2.0, 0.0]);
    }
}
