//! OptiFine Connected Textures (`optifine/ctm/**/*.properties`), as `ctm.properties` documents
//! them: the `ctm` (47 tiles), `ctm_compact` (5 tiles, drawn in quarters), `horizontal`,
//! `vertical`, `top`, `random`, `repeat` and `fixed` methods, matched by tile or by block, with
//! `connect`, `faces`, `heights`, `weights`, `symmetry` and `innerSeams`. The overlay methods,
//! `horizontal+vertical`, `vertical+horizontal` and rules limited to `biomes` are left out.
//!
//! Written from OptiFine's public documentation and its tile templates; no mod's source was read.

use crate::sprites::Sprites;
use horizon_optifine::{BlockMatch, Properties, RangeList, properties_files, resolve};
use std::collections::HashMap;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Method {
    Ctm,
    Compact,
    Horizontal,
    Vertical,
    Top,
    Random,
    Repeat(u32, u32),
    Fixed,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Connect {
    Block,
    Tile,
    State,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Symmetry {
    None,
    Opposite,
    All,
}

#[derive(Clone, Debug)]
pub struct Rule {
    pub method: Method,
    /// The tiles' sprite layers.
    pub tiles: Vec<u32>,
    pub connect: Connect,
    /// Faces it applies to, a bit per direction (down, up, north, south, west, east).
    pub faces: u8,
    pub blocks: Vec<BlockMatch>,
    pub tile_layers: Vec<u32>,
    pub weights: Vec<u32>,
    pub heights: Option<RangeList>,
    pub symmetry: Symmetry,
    pub inner_seams: bool,
    /// `ctm_compact`'s `ctm.<case>=<tile>` replacements.
    pub compact_cases: HashMap<u8, usize>,
}

/// The rules of the enabled packs, tile rules before block rules, each set in the order
/// OptiFine applies them (higher `weight` first, then by file name).
#[derive(Clone, Debug, Default)]
pub struct Rules {
    pub rules: Vec<Rule>,
    /// A sprite layer's first tile rule.
    pub by_tile: HashMap<u32, usize>,
}

impl Rules {
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

/// `ctm_template.png`: the tile for each set of connections, a bit each for up, down, left,
/// right, up-left, up-right, down-left and down-right (diagonals count only when both of their
/// sides connect).
pub const CTM_TILES: [u8; 256] = [
    0, 36, 12, 24, 3, 17, 5, 19, 1, 16, 4, 6, 2, 18, 7, 46, 0, 36, 12, 24, 3, 39, 5, 41, 1, 16, 4, 6, 2, 42, 7, 20, 0, 36,
    12, 24, 3, 17, 5, 19, 1, 37, 4, 30, 2, 40, 7, 8, 0, 36, 12, 24, 3, 39, 5, 41, 1, 37, 4, 30, 2, 38, 7, 11, 0, 36, 12, 24,
    3, 17, 15, 43, 1, 16, 4, 6, 2, 18, 29, 21, 0, 36, 12, 24, 3, 39, 15, 27, 1, 16, 4, 6, 2, 42, 29, 10, 0, 36, 12, 24, 3,
    17, 15, 43, 1, 37, 4, 30, 2, 40, 29, 34, 0, 36, 12, 24, 3, 39, 15, 27, 1, 37, 4, 30, 2, 38, 29, 32, 0, 36, 12, 24, 3,
    17, 5, 19, 1, 16, 13, 28, 2, 18, 31, 9, 0, 36, 12, 24, 3, 39, 5, 41, 1, 16, 13, 28, 2, 42, 31, 35, 0, 36, 12, 24, 3,
    17, 5, 19, 1, 37, 13, 25, 2, 40, 31, 23, 0, 36, 12, 24, 3, 39, 5, 41, 1, 37, 13, 25, 2, 38, 31, 33, 0, 36, 12, 24, 3,
    17, 15, 43, 1, 16, 13, 28, 2, 18, 14, 22, 0, 36, 12, 24, 3, 39, 15, 27, 1, 16, 13, 28, 2, 42, 14, 44, 0, 36, 12, 24, 3,
    17, 15, 43, 1, 37, 13, 25, 2, 40, 14, 45, 0, 36, 12, 24, 3, 39, 15, 27, 1, 37, 13, 25, 2, 38, 14, 26,
];

pub const UP: u8 = 1;
pub const DOWN: u8 = 2;
pub const LEFT: u8 = 4;
pub const RIGHT: u8 = 8;
pub const UP_LEFT: u8 = 16;
pub const UP_RIGHT: u8 = 32;
pub const DOWN_LEFT: u8 = 64;
pub const DOWN_RIGHT: u8 = 128;

/// A face's texture axes in the world: its right and its down, as the faces of a full block
/// map their texture (`[0, 0, 16, 16]` UVs). Faces in the order of the `faces` bits.
pub const AXES: [([i32; 3], [i32; 3]); 6] = [
    ([1, 0, 0], [0, 0, -1]),
    ([1, 0, 0], [0, 0, 1]),
    ([-1, 0, 0], [0, -1, 0]),
    ([1, 0, 0], [0, -1, 0]),
    ([0, 0, 1], [0, -1, 0]),
    ([0, 0, -1], [0, -1, 0]),
];

/// The outward normal of face `f` (down, up, north, south, west, east).
pub const NORMALS: [[i32; 3]; 6] = [[0, -1, 0], [0, 1, 0], [0, 0, -1], [0, 0, 1], [-1, 0, 0], [1, 0, 0]];

/// What the mesher draws for a face a rule matched.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pick {
    Tile(u32),
    /// `ctm_compact`: the layer of each texture quarter (top-left, top-right, bottom-left,
    /// bottom-right).
    Quarters([u32; 4]),
}

/// Reads every CTM rule of the packs under `root`, its tiles added to `sprites`.
pub fn load(root: &Path, sprites: &mut Sprites) -> Rules {
    let mut tile_rules = Vec::new();
    let mut block_rules = Vec::new();
    for file in properties_files(root, "ctm") {
        let Ok(text) = std::fs::read_to_string(root.join(&file)) else { continue };
        let props = Properties::parse(&text);
        let weight: i32 = props.get("weight").and_then(|w| w.trim().parse().ok()).unwrap_or(0);
        if let Some(rule) = rule(root, &file, &props, sprites) {
            let entry = (-weight, file.clone(), rule);
            if entry.2.tile_layers.is_empty() {
                block_rules.push(entry);
            } else {
                tile_rules.push(entry);
            }
        }
    }
    tile_rules.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
    block_rules.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
    let mut out = Rules::default();
    for (_, _, r) in tile_rules.into_iter().chain(block_rules) {
        let i = out.rules.len();
        for &l in &r.tile_layers {
            out.by_tile.entry(l).or_insert(i);
        }
        out.rules.push(r);
    }
    if !out.rules.is_empty() {
        log::info!("Connected textures: {} rule(s) from the resource packs", out.rules.len());
    }
    out
}

/// A tile's sprite name for a resource path (`assets/ns/textures/block/x.png` is `ns:block/x`,
/// other files of the namespace are named from its `textures/` folder).
fn sprite_name(path: &str) -> Option<String> {
    let rest = path.strip_prefix("assets/")?;
    let (ns, rest) = rest.split_once('/')?;
    let rest = rest.strip_suffix(".png").unwrap_or(rest);
    Some(match rest.strip_prefix("textures/") {
        Some(t) => format!("{ns}:{t}"),
        None => format!("{ns}:../{rest}"),
    })
}

fn tile_paths(file: &str, value: &str) -> Vec<Option<String>> {
    let mut out = Vec::new();
    for token in value.split_whitespace() {
        if let Some((a, b)) = token.split_once('-')
            && let (Ok(a), Ok(b)) = (a.parse::<u32>(), b.parse::<u32>())
        {
            for n in a..=b.min(a + 1024) {
                out.push(resolve(file, &format!("./{n}.png")));
            }
            continue;
        }
        if token == "<skip>" || token == "<default>" {
            out.push(None);
            continue;
        }
        let name = if token.ends_with(".png") { token.to_owned() } else { format!("{token}.png") };
        let full = if token.contains('/') && !token.starts_with("./") && !token.starts_with("~/") && !token.contains(':') {
            let ns = file.strip_prefix("assets/").and_then(|p| p.split('/').next()).unwrap_or("minecraft");
            Some(format!("assets/{ns}/{name}"))
        } else {
            resolve(file, &name)
        };
        out.push(full);
    }
    out
}

fn rule(root: &Path, file: &str, props: &Properties, sprites: &mut Sprites) -> Option<Rule> {
    if props.get("biomes").is_some_and(|b| !b.trim().is_empty()) {
        return None;
    }
    let method = match props.get("method").unwrap_or("default").trim() {
        "ctm" | "glass" => Method::Ctm,
        "ctm_compact" => Method::Compact,
        "horizontal" | "bookshelf" => Method::Horizontal,
        "vertical" => Method::Vertical,
        "top" | "sandstone" => Method::Top,
        "random" => Method::Random,
        "fixed" | "static" => Method::Fixed,
        "repeat" | "pattern" => {
            let w = props.get("width")?.trim().parse().ok()?;
            let h = props.get("height")?.trim().parse().ok()?;
            Method::Repeat(w, h)
        }
        _ => return None,
    };
    let mut tiles = Vec::new();
    for path in tile_paths(file, props.get("tiles")?) {
        let path = path?;
        if !root.join(&path).is_file() {
            return None;
        }
        tiles.push(sprites.insert(root, &sprite_name(&path)?).ok()?);
    }
    let needed = match method {
        Method::Ctm => 47,
        Method::Compact => 5,
        Method::Horizontal | Method::Vertical => 4,
        Method::Repeat(w, h) => (w * h) as usize,
        Method::Top | Method::Random | Method::Fixed => 1,
    };
    if tiles.len() < needed {
        return None;
    }
    let stem = file.rsplit('/').next()?.strip_suffix(".properties")?;
    let mut blocks = props.get("matchBlocks").map(BlockMatch::parse_list).unwrap_or_default();
    let mut tile_names: Vec<String> = props
        .get("matchTiles")
        .map(|v| v.split_whitespace().map(str::to_owned).collect())
        .unwrap_or_default();
    if blocks.is_empty() && tile_names.is_empty() {
        match stem.strip_prefix("block_") {
            Some(b) => blocks = BlockMatch::parse_list(b),
            None => tile_names.push(stem.to_owned()),
        }
    }
    let tile_layers: Vec<u32> = tile_names
        .iter()
        .filter_map(|t| {
            let t = t.strip_suffix(".png").unwrap_or(t);
            let name = if let Some((ns, rest)) = t.split_once(':') {
                if rest.contains('/') { format!("{ns}:{rest}") } else { format!("{ns}:block/{rest}") }
            } else if t.contains('/') {
                format!("minecraft:../{t}").replace("../textures/", "")
            } else {
                format!("minecraft:block/{t}")
            };
            sprites.layers.get(&name).copied()
        })
        .collect();
    if tile_layers.is_empty() && blocks.is_empty() {
        return None;
    }
    let connect = match props.get("connect").map(str::trim) {
        Some("block") => Connect::Block,
        Some("state") => Connect::State,
        Some("tile") => Connect::Tile,
        _ if tile_layers.is_empty() => Connect::Block,
        _ => Connect::Tile,
    };
    let faces = props.get("faces").map_or(0b11_1111, |v| {
        v.split_whitespace().fold(0u8, |m, f| {
            m | match f {
                "bottom" => 1,
                "top" => 2,
                "north" => 4,
                "south" => 8,
                "west" => 16,
                "east" => 32,
                "sides" => 0b11_1100,
                "all" => 0b11_1111,
                _ => 0,
            }
        })
    });
    let weights = props
        .get("weights")
        .map(|v| v.split_whitespace().filter_map(|w| w.parse().ok()).collect())
        .unwrap_or_default();
    let heights = props.get("heights").and_then(RangeList::parse).or_else(|| {
        let min = props.get("minHeight").and_then(|v| v.trim().parse::<i32>().ok());
        let max = props.get("maxHeight").and_then(|v| v.trim().parse::<i32>().ok());
        (min.is_some() || max.is_some()).then(|| RangeList(vec![(min.unwrap_or(i32::MIN), max.unwrap_or(i32::MAX))]))
    });
    let symmetry = match props.get("symmetry").map(str::trim) {
        Some("opposite") => Symmetry::Opposite,
        Some("all") => Symmetry::All,
        _ => Symmetry::None,
    };
    let compact_cases = props
        .with_prefix("ctm.")
        .filter_map(|(k, v)| Some((k.parse::<u8>().ok()?, v.trim().parse::<usize>().ok()?)))
        .filter(|(c, t)| *c < 47 && *t < tiles.len())
        .collect();
    Some(Rule {
        method,
        tiles,
        connect,
        faces,
        blocks,
        tile_layers,
        weights,
        heights,
        symmetry,
        inner_seams: props.get("innerSeams").is_some_and(|v| v.trim() == "true"),
        compact_cases,
    })
}

fn hash(x: i32, y: i32, z: i32, salt: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x8DA6_B343)
        ^ (y as u32).wrapping_mul(0xD816_3841)
        ^ (z as u32).wrapping_mul(0xCB1A_B31F)
        ^ salt.wrapping_mul(0x9E37_79B9);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^ (h >> 15)
}

impl Rule {
    /// The tile for face `f` of the block at `world`; `connects(offset)` says whether the block
    /// at that offset connects to this one.
    pub fn pick(&self, f: usize, world: [i32; 3], connects: &dyn Fn([i32; 3]) -> bool) -> Option<Pick> {
        if self.faces & (1 << f) == 0 {
            return None;
        }
        if let Some(h) = &self.heights
            && !h.contains(world[1])
        {
            return None;
        }
        let (right, down) = AXES[f];
        let normal = NORMALS[f];
        let add = |a: [i32; 3], b: [i32; 3], k: i32| [a[0] + b[0] * k, a[1] + b[1] * k, a[2] + b[2] * k];
        let joined = |du: i32, dv: i32| {
            let o = add(add([0; 3], right, du), down, dv);
            connects(o) && (self.inner_seams || !connects(add(o, normal, 1)))
        };
        let tile = |i: usize| self.tiles.get(i).copied();
        match self.method {
            Method::Fixed => tile(0).map(Pick::Tile),
            Method::Top => {
                if f < 2 {
                    return None;
                }
                connects([0, 1, 0]).then(|| tile(0)).flatten().map(Pick::Tile)
            }
            Method::Horizontal => {
                let (l, r) = (joined(-1, 0), joined(1, 0));
                tile(match (l, r) {
                    (true, true) => 1,
                    (false, true) => 0,
                    (true, false) => 2,
                    (false, false) => 3,
                })
                .map(Pick::Tile)
            }
            Method::Vertical => {
                let (u, d) = (joined(0, -1), joined(0, 1));
                tile(match (u, d) {
                    (true, true) => 1,
                    (true, false) => 0,
                    (false, true) => 2,
                    (false, false) => 3,
                })
                .map(Pick::Tile)
            }
            Method::Random => {
                let face = match self.symmetry {
                    Symmetry::All => 0,
                    Symmetry::Opposite => (f / 2) as u32,
                    Symmetry::None => f as u32,
                };
                let r = hash(world[0], world[1], world[2], face);
                let n = self.tiles.len();
                let i = if self.weights.len() == n && self.weights.iter().any(|w| *w > 0) {
                    let total: u32 = self.weights.iter().sum();
                    let mut x = r % total;
                    self.weights.iter().position(|w| {
                        if x < *w {
                            true
                        } else {
                            x -= w;
                            false
                        }
                    })?
                } else {
                    (r % n as u32) as usize
                };
                tile(i).map(Pick::Tile)
            }
            Method::Repeat(w, h) => {
                let dot = |a: [i32; 3]| a[0] * world[0] + a[1] * world[1] + a[2] * world[2];
                let mut u = dot(right);
                let v = dot(down);
                if self.symmetry == Symmetry::Opposite && matches!(f, 3 | 5) {
                    u = -u - 1;
                }
                let i = u.rem_euclid(w as i32) + v.rem_euclid(h as i32) * w as i32;
                tile(i as usize).map(Pick::Tile)
            }
            Method::Ctm | Method::Compact => {
                let mut m = 0u8;
                for (bit, du, dv) in [
                    (UP, 0, -1),
                    (DOWN, 0, 1),
                    (LEFT, -1, 0),
                    (RIGHT, 1, 0),
                    (UP_LEFT, -1, -1),
                    (UP_RIGHT, 1, -1),
                    (DOWN_LEFT, -1, 1),
                    (DOWN_RIGHT, 1, 1),
                ] {
                    if joined(du, dv) {
                        m |= bit;
                    }
                }
                let case = CTM_TILES[m as usize];
                if self.method == Method::Ctm {
                    return tile(case as usize).map(Pick::Tile);
                }
                if let Some(&t) = self.compact_cases.get(&case) {
                    return tile(t).map(Pick::Tile);
                }
                let quarter = |side_v: u8, side_u: u8, diag: u8| -> usize {
                    match (m & side_v != 0, m & side_u != 0) {
                        (false, false) => 0,
                        (true, false) => 2,
                        (false, true) => 3,
                        (true, true) if m & diag != 0 => 1,
                        (true, true) => 4,
                    }
                };
                let q = [
                    quarter(UP, LEFT, UP_LEFT),
                    quarter(UP, RIGHT, UP_RIGHT),
                    quarter(DOWN, LEFT, DOWN_LEFT),
                    quarter(DOWN, RIGHT, DOWN_RIGHT),
                ];
                if q.iter().all(|&t| t == q[0]) {
                    return tile(q[0]).map(Pick::Tile);
                }
                Some(Pick::Quarters([tile(q[0])?, tile(q[1])?, tile(q[2])?, tile(q[3])?]))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_corners() {
        assert_eq!(CTM_TILES[0], 0);
        assert_eq!(CTM_TILES[RIGHT as usize], 1);
        assert_eq!(CTM_TILES[(LEFT | RIGHT) as usize], 2);
        assert_eq!(CTM_TILES[LEFT as usize], 3);
        assert_eq!(CTM_TILES[DOWN as usize], 12);
        assert_eq!(CTM_TILES[(UP | DOWN) as usize], 24);
        assert_eq!(CTM_TILES[UP as usize], 36);
        assert_eq!(CTM_TILES[255], 26);
        assert_eq!(CTM_TILES[15], 46);
        assert_eq!(CTM_TILES[(DOWN | RIGHT | DOWN_RIGHT) as usize], 13);
        assert_eq!(CTM_TILES[(DOWN | RIGHT) as usize], 4);
        let distinct: std::collections::BTreeSet<u8> = CTM_TILES.iter().copied().collect();
        assert_eq!(distinct.len(), 47);
    }

    #[test]
    fn tile_names() {
        assert_eq!(sprite_name("assets/minecraft/textures/block/glass.png").as_deref(), Some("minecraft:block/glass"));
        assert_eq!(
            sprite_name("assets/minecraft/optifine/ctm/glass/0.png").as_deref(),
            Some("minecraft:../optifine/ctm/glass/0")
        );
        let paths = tile_paths("assets/minecraft/optifine/ctm/glass/glass.properties", "0-2 x");
        assert_eq!(paths.len(), 4);
        assert_eq!(paths[1].as_deref(), Some("assets/minecraft/optifine/ctm/glass/1.png"));
        assert_eq!(paths[3].as_deref(), Some("assets/minecraft/optifine/ctm/glass/x.png"));
    }

    #[test]
    fn horizontal_and_random() {
        let mut r = Rule {
            method: Method::Horizontal,
            tiles: vec![10, 11, 12, 13],
            connect: Connect::Block,
            faces: 0b11_1111,
            blocks: Vec::new(),
            tile_layers: vec![1],
            weights: Vec::new(),
            heights: None,
            symmetry: Symmetry::None,
            inner_seams: false,
            compact_cases: HashMap::new(),
        };
        let only_east = |o: [i32; 3]| o == [1, 0, 0];
        assert_eq!(r.pick(3, [0, 0, 0], &only_east), Some(Pick::Tile(10)));
        assert_eq!(r.pick(2, [0, 0, 0], &only_east), Some(Pick::Tile(12)));
        r.method = Method::Random;
        let a = r.pick(3, [5, 64, 9], &|_| false);
        assert_eq!(a, r.pick(3, [5, 64, 9], &|_| false));
    }
}
