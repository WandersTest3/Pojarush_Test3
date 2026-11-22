//! `block.properties` (spec 4.1): `block.<id>=<block> <block> ...`, each block
//! `[namespace:]name` with optional state filters `:<property>=<v1>,<v2>`. Values in one filter
//! are alternatives; separate filters must all match. Names the game does not know are simply
//! never asked for. Block tags (`%tag`, tier E) are not read yet.

use crate::Properties;

#[derive(Clone, Debug, PartialEq)]
struct Rule {
    id: i32,
    /// `minecraft:stone`.
    name: String,
    filters: Vec<(String, Vec<String>)>,
}

/// The ids `block.properties` gives block states.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BlockIds {
    rules: Vec<Rule>,
    /// Lines skipped and why.
    pub notes: Vec<String>,
}

impl BlockIds {
    pub fn parse(properties: &Properties) -> Self {
        Self::parse_kind(properties, "block")
    }

    /// The same for `<kind>.<id>=` lines: `entity` in `entity.properties`, `item` in
    /// `item.properties` (4.1); names there carry no state filters.
    pub fn parse_kind(properties: &Properties, kind: &str) -> Self {
        let mut out = Self::default();
        let prefix = format!("{kind}.");
        for e in &properties.entries {
            let Some(id) = e.key.strip_prefix(prefix.as_str()) else {
                continue;
            };
            let Ok(id) = id.parse::<i32>() else {
                // `layer.*` (4.2) and other keys are not ids.
                continue;
            };
            for word in e.value.split_whitespace() {
                if word == "=" || word.starts_with('=') {
                    continue;
                }
                if word.starts_with('%') {
                    out.notes.push(format!(
                        "line {}: block tags are not read yet: {word}",
                        e.line
                    ));
                    continue;
                }
                let (names, filters): (Vec<&str>, Vec<&str>) =
                    word.split(':').partition(|p| !p.contains('='));
                let name = match names[..] {
                    [name] => format!("minecraft:{name}"),
                    [namespace, name] => format!("{namespace}:{name}"),
                    _ => {
                        out.notes
                            .push(format!("line {}: not a block name: {word}", e.line));
                        continue;
                    }
                };
                let filters = filters
                    .iter()
                    .filter_map(|f| f.split_once('='))
                    .map(|(k, v)| (k.to_owned(), v.split(',').map(str::to_owned).collect()))
                    .collect();
                out.rules.push(Rule { id, name, filters });
            }
        }
        out
    }

    /// The id of block `name` (`minecraft:stone`) in the state with `properties`. The rule with
    /// the most filters that all match wins, then the first in the file (one name belongs to
    /// one id; a filtered entry is the more specific one).
    pub fn id<'a>(&self, name: &str, properties: impl Fn(&str) -> Option<&'a str>) -> Option<i32> {
        self.rules
            .iter()
            .filter(|r| r.name == name)
            .filter(|r| {
                r.filters
                    .iter()
                    .all(|(k, vs)| properties(k).is_some_and(|v| vs.iter().any(|x| x == v)))
            })
            .min_by_key(|r| std::cmp::Reverse(r.filters.len()))
            .map(|r| r.id)
    }

    /// Every name without state filters and its id (the first line naming it wins): what
    /// `entity.properties` and `item.properties` hold.
    pub fn plain(&self) -> std::collections::HashMap<String, i32> {
        let mut out = std::collections::HashMap::new();
        for r in self.rules.iter().filter(|r| r.filters.is_empty()) {
            out.entry(r.name.clone()).or_insert(r.id);
        }
        out
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

const COLORS: [&str; 16] = [
    "white", "orange", "magenta", "light_blue", "yellow", "lime", "pink", "gray", "light_gray",
    "cyan", "purple", "blue", "brown", "green", "red", "black",
];
const WOODS: [&str; 6] = ["oak", "spruce", "birch", "jungle", "acacia", "dark_oak"];

/// The `block.properties` every pack without its own gets, as OptiFine and Iris give it: the
/// block ids of Minecraft 1.12, which the old packs test (`mc_Entity.x == 8.0` is water),
/// under the blocks' current names.
pub fn legacy_block_properties() -> String {
    let mut lines: Vec<(u32, Vec<String>)> = Vec::new();
    let mut add = |id: u32, names: &[&str]| lines.push((id, names.iter().map(|n| (*n).to_owned()).collect()));
    add(1, &["stone", "granite", "diorite", "andesite", "polished_granite", "polished_diorite", "polished_andesite"]);
    add(2, &["grass_block"]);
    add(3, &["dirt", "coarse_dirt", "podzol", "rooted_dirt"]);
    add(4, &["cobblestone"]);
    add(7, &["bedrock"]);
    add(8, &["water"]);
    add(10, &["lava"]);
    add(12, &["sand", "red_sand"]);
    add(13, &["gravel"]);
    add(14, &["gold_ore"]);
    add(15, &["iron_ore"]);
    add(16, &["coal_ore"]);
    add(19, &["sponge", "wet_sponge"]);
    add(20, &["glass"]);
    add(21, &["lapis_ore"]);
    add(22, &["lapis_block"]);
    add(24, &["sandstone", "chiseled_sandstone", "cut_sandstone", "smooth_sandstone"]);
    add(25, &["note_block"]);
    add(30, &["cobweb"]);
    add(31, &["short_grass", "grass", "fern"]);
    add(32, &["dead_bush"]);
    add(37, &["dandelion"]);
    add(38, &["poppy", "blue_orchid", "allium", "azure_bluet", "red_tulip", "orange_tulip", "white_tulip", "pink_tulip", "oxeye_daisy", "cornflower", "lily_of_the_valley"]);
    add(39, &["brown_mushroom"]);
    add(40, &["red_mushroom"]);
    add(41, &["gold_block"]);
    add(42, &["iron_block"]);
    add(45, &["bricks"]);
    add(46, &["tnt"]);
    add(47, &["bookshelf"]);
    add(48, &["mossy_cobblestone"]);
    add(49, &["obsidian"]);
    add(50, &["torch", "wall_torch"]);
    add(51, &["fire"]);
    add(52, &["spawner"]);
    add(54, &["chest", "trapped_chest"]);
    add(56, &["diamond_ore"]);
    add(57, &["diamond_block"]);
    add(58, &["crafting_table"]);
    add(59, &["wheat"]);
    add(60, &["farmland"]);
    add(61, &["furnace"]);
    add(62, &["furnace:lit=true"]);
    add(73, &["redstone_ore"]);
    add(74, &["redstone_ore:lit=true"]);
    add(76, &["redstone_torch", "redstone_wall_torch"]);
    add(78, &["snow"]);
    add(79, &["ice", "frosted_ice"]);
    add(80, &["snow_block"]);
    add(81, &["cactus"]);
    add(82, &["clay"]);
    add(83, &["sugar_cane"]);
    add(84, &["jukebox"]);
    add(86, &["pumpkin", "carved_pumpkin"]);
    add(87, &["netherrack"]);
    add(88, &["soul_sand"]);
    add(89, &["glowstone"]);
    add(90, &["nether_portal"]);
    add(91, &["jack_o_lantern"]);
    add(95, &[]);
    add(98, &["stone_bricks", "mossy_stone_bricks", "cracked_stone_bricks", "chiseled_stone_bricks"]);
    add(103, &["melon"]);
    add(104, &["pumpkin_stem", "attached_pumpkin_stem"]);
    add(105, &["melon_stem", "attached_melon_stem"]);
    add(106, &["vine"]);
    add(110, &["mycelium"]);
    add(111, &["lily_pad"]);
    add(112, &["nether_bricks"]);
    add(115, &["nether_wart"]);
    add(116, &["enchanting_table"]);
    add(119, &["end_portal"]);
    add(121, &["end_stone"]);
    add(123, &["redstone_lamp"]);
    add(124, &["redstone_lamp:lit=true"]);
    add(129, &["emerald_ore"]);
    add(133, &["emerald_block"]);
    add(138, &["beacon"]);
    add(141, &["carrots"]);
    add(142, &["potatoes"]);
    add(152, &["redstone_block"]);
    add(153, &["nether_quartz_ore"]);
    add(155, &["quartz_block", "chiseled_quartz_block", "quartz_pillar", "smooth_quartz"]);
    add(165, &["slime_block"]);
    add(169, &["sea_lantern"]);
    add(170, &["hay_block"]);
    add(173, &["coal_block"]);
    add(174, &["packed_ice", "blue_ice"]);
    add(175, &["sunflower", "lilac", "tall_grass", "large_fern", "rose_bush", "peony"]);
    add(198, &["end_rod"]);
    add(213, &["magma_block"]);
    add(214, &["nether_wart_block"]);
    add(215, &["red_nether_bricks"]);
    add(216, &["bone_block"]);
    let mut out = String::new();
    for (id, names) in &lines {
        if !names.is_empty() {
            out.push_str(&format!("block.{id}={}\n", names.join(" ")));
        }
    }
    let family = |id: u32, suffix: &str| -> String {
        let names: Vec<String> = COLORS.iter().map(|c| format!("{c}_{suffix}")).collect();
        format!("block.{id}={}\n", names.join(" "))
    };
    out.push_str(&family(35, "wool"));
    out.push_str(&family(95, "stained_glass"));
    out.push_str(&family(160, "stained_glass_pane"));
    out.push_str(&family(159, "terracotta"));
    out.push_str(&family(251, "concrete"));
    let wood = |id: u32, kind: &str, of: &[&str]| -> String {
        let names: Vec<String> = of.iter().map(|w| format!("{w}_{kind}")).collect();
        format!("block.{id}={}\n", names.join(" "))
    };
    out.push_str(&wood(5, "planks", &WOODS));
    out.push_str(&wood(6, "sapling", &WOODS));
    out.push_str(&wood(17, "log", &WOODS[..4]));
    out.push_str(&wood(18, "leaves", &WOODS[..4]));
    out.push_str(&wood(161, "leaves", &WOODS[4..]));
    out.push_str(&wood(162, "log", &WOODS[4..]));
    out.push_str("block.102=glass_pane\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Macros;

    #[test]
    fn names_states_and_precedence() {
        let text = "block.10=stone minecraft:dirt mod:thing \\\n grass_block:snowy=false\n\
                    block.11=grass_block\nblock.12=oak_leaves:persistent=true:distance=1,2 %leaves\n\
                    layer.translucent=glass\n";
        let ids = BlockIds::parse(&Properties::parse(text, &Macros::standard()));
        let none = |_: &str| None;
        assert_eq!(ids.id("minecraft:stone", none), Some(10));
        assert_eq!(ids.id("mod:thing", none), Some(10));
        assert_eq!(ids.id("minecraft:glass", none), None);
        let snowy = |s: &'static str| move |k: &str| (k == "snowy").then_some(s);
        // The filtered entry is the more specific one.
        assert_eq!(ids.id("minecraft:grass_block", snowy("false")), Some(10));
        assert_eq!(ids.id("minecraft:grass_block", snowy("true")), Some(11));
        let leaves = |p: &'static str, d: &'static str| {
            move |k: &str| match k {
                "persistent" => Some(p),
                "distance" => Some(d),
                _ => None,
            }
        };
        assert_eq!(
            ids.id("minecraft:oak_leaves", leaves("true", "2")),
            Some(12)
        );
        assert_eq!(ids.id("minecraft:oak_leaves", leaves("true", "3")), None);
        assert_eq!(ids.id("minecraft:oak_leaves", leaves("false", "1")), None);
        assert_eq!(ids.notes.len(), 1, "{:?}", ids.notes);
    }

    #[test]
    fn entity_lines_by_name() {
        let text = "entity.10=zombie minecraft:husk\nentity.11=zombie mod:thing\nblock.5=stone\n";
        let ids = BlockIds::parse_kind(&Properties::parse(text, &Macros::standard()), "entity");
        let plain = ids.plain();
        assert_eq!(
            plain["minecraft:zombie"], 10,
            "the first line naming it wins"
        );
        assert_eq!(plain["minecraft:husk"], 10);
        assert_eq!(plain["mod:thing"], 11);
        assert!(!plain.contains_key("minecraft:stone"));
    }

    #[test]
    fn legacy_ids_give_water_and_leaves() {
        let ids = BlockIds::parse(&Properties::parse(&legacy_block_properties(), &Macros::standard()));
        let none = |_: &str| None;
        assert_eq!(ids.id("minecraft:water", none), Some(8));
        assert_eq!(ids.id("minecraft:ice", none), Some(79));
        assert_eq!(ids.id("minecraft:oak_leaves", none), Some(18));
        assert_eq!(ids.id("minecraft:acacia_leaves", none), Some(161));
        assert_eq!(ids.id("minecraft:tall_grass", none), Some(175));
        assert_eq!(ids.id("minecraft:glowstone", none), Some(89));
        assert!(ids.notes.is_empty(), "{:?}", ids.notes);
    }
}
