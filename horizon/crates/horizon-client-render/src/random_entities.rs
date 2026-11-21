//! OptiFine Random Entities (`random_entities.properties`): a mob's texture `entity/x/y` drawn as
//! one of `entity/x/y2`, `y3`... that the packs add, chosen by the mob, either evenly or by the
//! rules of `optifine/random/entity/x/y.properties` (`textures.N`/`skins.N`, `weights.N`, with the
//! `name`, `heights`, `baby` and `health` tests; a rule with another test is skipped). The
//! variants live in `optifine/random/entity/...` (or `mcpatcher/mob/...`) or beside the texture,
//! and the entity atlas takes them under their texture's folder.
//!
//! Written from OptiFine's public documentation; no mod's source was read.

use horizon_optifine::{Name, Properties, RangeList};
use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;

#[derive(Debug)]
struct Rule {
    textures: Vec<u32>,
    weights: Vec<u32>,
    name: Option<Name>,
    heights: Option<RangeList>,
    baby: Option<bool>,
    health: Option<RangeList>,
}

/// Each texture's rules (`entity/creeper/creeper`), from the enabled packs.
static RULES: RwLock<Option<HashMap<String, Vec<Rule>>>> = RwLock::new(None);

/// Reads the rules of the packs under `root` again (a pack reload).
pub fn reload(root: &Path) {
    let mut out = HashMap::new();
    for top in ["optifine/random/entity", "mcpatcher/mob"] {
        let dir = root.join("assets/minecraft").join(top);
        let mut files = Vec::new();
        walk(&dir, &mut files, 0);
        for file in files {
            let Ok(rel) = file.strip_prefix(&dir) else { continue };
            let rel = rel.with_extension("").to_string_lossy().replace('\\', "/");
            let Ok(text) = std::fs::read_to_string(&file) else { continue };
            let rules = parse(&Properties::parse(&text));
            if !rules.is_empty() {
                out.insert(format!("entity/{rel}"), rules);
            }
        }
    }
    if let Ok(mut r) = RULES.write() {
        *r = (!out.is_empty()).then_some(out);
    }
}

fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>, depth: usize) {
    if depth > 8 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out, depth + 1);
        } else if p.extension().is_some_and(|x| x == "properties") {
            out.push(p);
        }
    }
}

fn parse(props: &Properties) -> Vec<Rule> {
    let mut indices: Vec<u32> = props
        .entries
        .iter()
        .filter_map(|(k, _)| {
            let (key, n) = k.rsplit_once('.')?;
            matches!(key, "textures" | "skins").then(|| n.parse().ok()).flatten()
        })
        .collect();
    indices.sort_unstable();
    indices.dedup();
    let known = ["textures", "skins", "weights", "name", "heights", "minHeight", "maxHeight", "baby", "health"];
    let mut out = Vec::new();
    for n in indices {
        let get = |k: &str| props.get(&format!("{k}.{n}"));
        if props.entries.iter().any(|(k, _)| {
            k.rsplit_once('.').is_some_and(|(key, i)| i == n.to_string() && !known.contains(&key))
        }) {
            continue;
        }
        let Some(list) = get("textures").or_else(|| get("skins")) else { continue };
        let textures: Vec<u32> = RangeList::parse(list)
            .map(|r| r.0.iter().flat_map(|&(a, b)| a.max(1)..=b.min(a.max(1) + 256)).map(|x| x as u32).collect())
            .unwrap_or_default();
        if textures.is_empty() {
            continue;
        }
        let heights = get("heights").and_then(RangeList::parse).or_else(|| {
            let min = get("minHeight").and_then(|v| v.trim().parse::<i32>().ok());
            let max = get("maxHeight").and_then(|v| v.trim().parse::<i32>().ok());
            (min.is_some() || max.is_some()).then(|| RangeList(vec![(min.unwrap_or(i32::MIN), max.unwrap_or(i32::MAX))]))
        });
        out.push(Rule {
            textures,
            weights: get("weights").map(|w| w.split_whitespace().filter_map(|x| x.parse().ok()).collect()).unwrap_or_default(),
            name: get("name").and_then(Name::parse),
            heights,
            baby: get("baby").map(|b| b.trim() == "true"),
            health: get("health").filter(|h| !h.contains('%')).and_then(RangeList::parse),
        });
    }
    out
}

/// What the rules test about a mob.
pub struct Subject<'a> {
    /// A number fixed for the mob (its UUID's or its id's bits).
    pub seed: u64,
    pub y: i32,
    pub name: Option<&'a str>,
    pub baby: bool,
    pub health: f32,
}

fn mix(mut x: u64) -> u64 {
    x ^= x >> 33;
    x = x.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    x ^= x >> 33;
    x = x.wrapping_mul(0xC4CE_B9FE_1A85_EC53);
    x ^ (x >> 33)
}

/// The texture `texture` is drawn with for `m`: a variant the atlas holds (`has`), or `None` to
/// keep `texture`.
pub fn pick(texture: &str, m: &Subject<'_>, has: &dyn Fn(&str) -> bool) -> Option<String> {
    let variant = |n: u32| (n > 1).then(|| format!("{texture}{n}")).filter(|t| has(t));
    let r = mix(m.seed);
    if let Ok(rules) = RULES.read()
        && let Some(rules) = rules.as_ref().and_then(|r| r.get(texture))
    {
        let rule = rules.iter().find(|rule| {
            rule.name.as_ref().is_none_or(|n| m.name.is_some_and(|name| n.matches(name)))
                && rule.heights.as_ref().is_none_or(|h| h.contains(m.y))
                && rule.baby.is_none_or(|b| b == m.baby)
                && rule.health.as_ref().is_none_or(|h| h.contains(m.health as i32))
        })?;
        let n = rule.textures.len();
        let i = if rule.weights.len() == n && rule.weights.iter().any(|w| *w > 0) {
            let total: u64 = rule.weights.iter().map(|w| u64::from(*w)).sum();
            let mut x = r % total;
            rule.weights
                .iter()
                .position(|w| {
                    let w = u64::from(*w);
                    if x < w {
                        true
                    } else {
                        x -= w;
                        false
                    }
                })
                .unwrap_or(0)
        } else {
            (r % n as u64) as usize
        };
        return variant(rule.textures[i]);
    }
    // Without rules: the texture and every numbered variant from 2 up, evenly.
    if !has(&format!("{texture}2")) {
        return None;
    }
    let mut count = 2;
    while count < 64 && has(&format!("{texture}{}", count + 1)) {
        count += 1;
    }
    variant((r % count as u64) as u32 + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_and_plain_variants() {
        let props = Properties::parse("textures.1=2-3\nweights.1=1 0\nbiomes.2=desert\ntextures.2=4\ntextures.3=1\n");
        let rules = parse(&props);
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].textures, vec![2, 3]);
        let has = |t: &str| matches!(t, "entity/cow/cow2" | "entity/cow/cow3");
        let m = Subject { seed: 7, y: 64, name: None, baby: false, health: 10.0 };
        let got = pick("entity/cow/cow", &m, &has);
        assert!(matches!(got.as_deref(), None | Some("entity/cow/cow2") | Some("entity/cow/cow3")));
        assert_eq!(pick("entity/pig/pig", &m, &has), None);
    }
}
