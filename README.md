# Pojarush_Test3

An Android launcher for a single game: **Horizon**, Minecraft: Java Edition 26.3 rewritten in Rust, running
natively on the phone (no JVM while playing).

- Horizon: https://github.com/horizon-rust/horizon (MIT), code in `horizon/`
- Mods for Pojarush: **https://github.com/WandersTest3/Pojarush_Test3-Mods** (the Pojarush Mods site:
  https://wanderstest3.github.io/Pojarush_Test3-Mods/). Learn more about the mods there.
- Horizon's newer wasm `.hmod` mods (api `"0.2"` with a `[wasm]` table, such as the official
  Keystrokes) run on 64-bit phones through Horizon's own wasm mod runtime, vendored in
  `horizon/vendor/horizon-wasm-mods` under the LGPL-3.0-or-later (see its README for what changed).
- Not an official Minecraft product, and no Mojang file ships with the app. You need to own Minecraft: Java Edition.

## Download

The launcher comes as two APKs. Install the one that matches your device:

| APK | Device | Code mods (WebAssembly) |
|---|---|---|
| `Pojarush-arm64-v8a` | 64-bit (most current phones) | Yes |
| `Pojarush-armeabi-v7a` | 32-bit | No (content mods only) |

Both use the same package and the same signature, so one updates over the other.

## How to use

1. Pick a player name or sign in with a Microsoft account.
2. Optional: allow access to the internal storage. Worlds, options, packs, controls and logs then live in
   `Pojarush_Test3/` at the root of the storage (without the permission, they stay in the app's folder).
3. Prepare the game files: the launcher downloads the preparation Java 25 (about 37 MB, once) and Minecraft 26.3
   from Mojang (about 330 MB, plus 380 MB with sounds), then runs the exporters for a few minutes.
4. Tap Play.

Controls: the Pojarush and Bedrock layouts that ship with the app, and any Pojarush (`pojarush`), Zalith
(`editorVersion`), Fold Craft Launcher (`viewGroups`) or PojavLauncher (`mControlDataList`) layout imported from a
`.json` or placed in `Pojarush_Test3/controls/`. Button images go in `Pojarush_Test3/hud_image/`, cursors in
`Pojarush_Test3/cursor_image/` and launcher fonts in `Pojarush_Test3/font/`.

## Game folders

Everything lives in `Pojarush_Test3/horizon/game/`:

| Folder | What goes in it |
|---|---|
| `horizon-mods/` | `.hmod` mods |
| `resourcepacks/` | Java Edition resource packs (`.zip` with `pack.mcmeta`) |
| `shaderpacks/` | `.hshader` shader packs and OptiFine/Iris-format packs (`.zip`) |
| `saves/`, `screenshots/` | Worlds and screenshots |
| `logs/` | `latest.log` (preparation and game in the same file, as in Minecraft) and the `.log.gz` of earlier runs |
| `crash-reports/` | Crash reports, only when the game really fails |

`latest.log` stays open while the game runs and is only compressed into `YYYY-MM-DD-N.log.gz` after the game closes.
With "Clear logs and crash reports regularly" on (the default), only the 3 most recent files of each are kept.

In **Other → Extras**, "Clear junk" deletes only caches (shaders and the app's) and leftovers of old versions. The
downloaded Minecraft, worlds, packs, mods, controls and settings are never touched.

The launcher's language is chosen in **Other → Interface → Language** (English, Brazilian and European Portuguese,
Spanish, French, German, Italian, Russian, Ukrainian, Polish, Turkish, Arabic, Simplified and Traditional Chinese,
Japanese, Korean, Thai, Vietnamese and Indonesian). In languages other than English and Portuguese, the long help texts
still show in English. Worlds, packs, mods, controls and settings are not touched.

The launcher's **Packs** page imports files into these folders and downloads from the Pojarush Mods site and
Horizon's registry (playhorizon.dev). Installed mods and shaders always load; to turn one off, take it out of the
folder.

# Tutorial: making mods

Fabric and NeoForge `.jar` files don't run in Horizon. Two kinds of mod run in the app:

| Kind | API | What it is | Where it runs |
|---|---|---|---|
| Content mod | `0.2` | Data and assets only: blocks, items, recipes, loot, tags, models, textures | 64 and 32 bits |
| Code mod | `0.1` | Rust compiled to WebAssembly (`mod.wasm`), isolated in a sandbox | 64 bits only |

Native mods (`.hnmod`) are compiled together with the game on a PC and don't run in the app.

Mods for Horizon's newer native SDK (`api = "0.2"` with a `[wasm]` section, such as Horizon's official Keystrokes
3.0.1) link against the exact Horizon build they were made for, so they cannot run in Pojarush's engine. Ports of
those mods to the Pojarush API live in `horizon/mods/` (for example `keystrokes-port`).

## 1. Tools

You need Rust installed and this repository cloned.

```sh
git clone https://github.com/WandersTest3/Pojarush_Test3
cd Pojarush_Test3/horizon
cargo install --path crates/cargo-horizon-mod
rustup target add wasm32-unknown-unknown
```

The `wasm32-unknown-unknown` target is only needed for code mods.

## 2. Content mod

```sh
cargo horizon-mod new my-mod --template content
cd my-mod
cargo horizon-mod build
cargo horizon-mod check target/horizon-mod/my_mod.hmod
```

The template creates a complete block (`glowing_amethyst`) with these files:

| File | Purpose |
|---|---|
| `horizon-mod.toml` | The mod's manifest |
| `data/<id>/horizon/block/<name>.json` | Declares the block from a vanilla block |
| `data/<id>/horizon/item/<name>.json` | Declares an item from a vanilla item |
| `assets/<id>/blockstates/`, `models/`, `items/`, `lang/` | Look and names, as in a resource pack |
| `data/<id>/recipe/`, `loot_table/` | Recipes and drops, as in a data pack |
| `data/minecraft/tags/` | Additions to vanilla tags (for example `mineable/pickaxe`) |

Manifest:

```toml
id = "my_mod"
name = "My mod"
version = "0.1.0"
description = "..."
authors = ["Your name"]
api = "0.2"
side = "both"
capabilities = ["registries"]
```

A block copies a vanilla block and changes what it needs:

```json
{ "template": "minecraft:iron_block",
  "properties": { "hardness": 4.0, "light": 3, "sounds": "minecraft:amethyst_block" },
  "item": true }
```

An item copies a vanilla item and replaces data components:

```json
{ "template": "minecraft:iron_sword",
  "components": {
    "minecraft:max_damage": 900,
    "!minecraft:repairable": {} } }
```

Rules: the files live in the mod's own namespace (`assets/<id>/`, `data/<id>/`), plus additions in
`data/minecraft/tags/` and `data/c/tags/`. Unknown keys are refused. Two enabled mods can't use the same namespace.
More in `horizon/docs/mods.md` and `horizon/docs/modpacks.md`.

## 3. Code mod (WebAssembly)

```sh
cargo horizon-mod new my-code --template code
cd my-code
```

In the generated `Cargo.toml`, point the SDK at this repository:

```toml
[dependencies]
horizon-mod-sdk = { path = "../Pojarush_Test3/horizon/crates/horizon-mod-sdk" }
```

Then:

```sh
cargo horizon-mod build
cargo horizon-mod check target/horizon-mod/my_code.hmod
```

`check` shows what the mod may do and runs it outside the game. With `--json`, its `output` lists what the mod did
during the trial run (chat lines, commands, swaps...).

Smallest example (`src/lib.rs`), a `/hello` command:

```rust
use horizon_mod_sdk::{Mod, Registration, chat};

struct MyMod {
    times: u32,
}

impl Mod for MyMod {
    fn init() -> Result<(Self, Registration), String> {
        Ok((MyMod { times: 0 }, Registration::new().command("hello")))
    }

    fn command(&mut self, _name: &str, args: &str) -> Result<Option<String>, String> {
        self.times += 1;
        chat::show(&format!("Hello {args}! ({} so far)", self.times));
        Ok(None)
    }
}

horizon_mod_sdk::export_mod!(MyMod);
```

Functions the game calls (all optional except `init`):

| Function | When | How to ask for it |
|---|---|---|
| `init()` | When the mod loads | Always |
| `tick(tick)` | 20 times a second | `Registration::new().on_tick()` |
| `frame(seconds)` | Every drawn frame | `.on_frame()` |
| `command(name, args)` | The player uses one of the mod's commands | `.command("name")` |
| `chat(text)` | A line reaches the chat | `.on_chat()` |
| `ui_select(id, row)` | A click on a button or row of the mod's screens | Capability `ui` |
| `ui_settings_opened()`, `ui_closed()` | The mod's settings page opens, a screen closes | Capability `ui` |

Capabilities (`capabilities` in the manifest). The player sees the list before turning the mod on, and the mod only
reaches what it asked for:

| Capability | Allows |
|---|---|
| `game-read` | Reading position, health, time and other game data |
| `chat` | Showing messages in the chat |
| `hud` | Drawing text on the HUD |
| `render` | Shapes on the HUD, camera shake and zoom |
| `input` | Its own keys, the mouse wheel, the use button on the mod's items, the inventory |
| `ui` | Its own screens and a settings page in Options > Mods |
| `storage` | Saving the mod's settings to disk |
| `command` | Running vanilla commands as the player |
| `recipes` | Reading recipes and items (recipe viewers) |
| `registries` | New blocks and items through the mod's `data/` folder |

`render`, `input`, `ui` and `recipes` may still change between versions: besides `capabilities`, they must be listed
in `experimental = [...]` in the manifest, as in the zoom mod.

Mod information, in the style of `fabric.mod.json` (all optional, Pojarush 1.11.17 or newer):

```toml
minecraft = "26.3"                 # Minecraft versions: "26.3", "26.x", ">=26.3", "*"
license = "Mod Test3 License V1.0"
icon = "assets/my_code/icon.png"   # PNG inside the .hmod, shown in the mod lists

[contact]
homepage = "https://..."
sources = "https://..."
issues = "https://..."
```

`cargo horizon-mod build` packs the icon. A mod made for another Minecraft version is refused with the version it was
made for.

API level 10 (capability `ui`): `ui::mods()` and `ui::open_mod_settings(id)` for mod lists; `ui::shader_packs()`,
`ui::shader_pack()`, `ui::set_shader_pack(name)`, `ui::open_shader_options()`, `ui::open_shader_packs()` and
`ui::reload_shader_pack()` for shaders.

Level 11: `ui::set_title_button(label)` puts a small mod icon next to the language and accessibility icons on the
title screen and in the pause menu (the click reaches `ui_select` with id `0xFFFFFF00`); `ui::set_video_button(label)`
puts a button at the top of Video Settings (id `0xFFFFFF01`); `ui::open_mod_list()` opens the game's Mod Menu screen
(list on the left, details on the right).

GeckoLib-style models (level 11, capability `render`): put the Bedrock geometry (`<name>.geo.json`, the format of
GeckoLib and Blockbench), the animations (`<name>.animation.json`) and the PNG texture (up to 128x128) in the mod's
`models/` folder. `cargo horizon-mod build` packs the folder. In code:

```rust
let id = render::spawn_model("totem.geo.json", "totem.png", ModelPlace { x, y, z, yaw, pitch: 0.0, scale: 1.0 })?;
render::play_animation(id, "totem.animation.json", "animation.totem.spin", true);
render::set_bone(id, "head", (0.0, 45.0, 0.0), true); // turn or hide a bone from code
render::place_model(id, new_place);
render::remove_model(id);
```

Models the mod builds itself and attaches to the player (level 11): `render::spawn_model_json(geometry, "player:skin",
place)` takes Bedrock geometry as text and the player's skin as the texture; `render::attach_model(id, "head")` attaches
the model to a part of the player (drawn in third person); `render::hide_player_parts(&["hat".into()])` hides parts of
the model; `render::player_skin()` gives the skin (64x64 RGBA) and `render::player_skin_slim()` says whether the arms
are slim. `render::set_player_cape("capes/x.png")` puts a 64x32 cape from the mod's `models/` folder on the player
(on the elytra too).

Level 12: `ui::open_screen_v3` and `ui::set_settings_v3` take every widget, including `entity`, which draws the player
(`"player"`) or an entity (`"minecraft:zombie"`) in 3D in a box, turned toward the cursor if `follow_cursor`.
`ui::set_resets(&[ids])` says which buttons and sliders go back to their default with Shift + click: the game calls
`ui_select(id, u32::MAX)`. In the shader options, Shift + click also puts an option back to the pack's default.

Level 13: `game::entities(distance)` lists the entities near the player as the frame shows them (id, type, position,
hitbox, eye height, where they look, hurt time, vehicle, whether they are targeted), `game::hitboxes_shown()` says
whether F3+B is on, `render::set_world_shapes(shapes, on_top)` draws lines, arrows, boxes and points in the world and
`render::hide_hitboxes(true)` takes away the game's own hitboxes (the Combat Hitboxes port in
`horizon/mods/combat-hitboxes` uses it). `entity` widgets up to 64 pixels high become tiles, four to a row;
`"cape:FILE"` draws a cape from the mod's `models/` folder in 3D; clicking an `entity` calls `ui_select(id, 0)`.
`"player:back"` shows the player from behind, and a `"column"` entity splits the screen as Mod Menu does: the widgets
before it stay in a fixed left column and the ones after it scroll in the right column.

Level 14: `input::inventory()` lists the player's stacks (0–8 hotbar, 9–35 inventory, 36–39 armor, 40 off hand) and
`input::swap_offhand(slot)` swaps a slot from 0 to 35 with the off hand, as the F key does in the inventory (PR Auto
Totem uses it).

Custom sky: the engine reads `assets/minecraft/optifine/sky/world0/skyN.properties` (and `mcpatcher/sky`, and
`world1` for the End) and FabricSkyboxes' `square-textured` skies in `assets/fabricskyboxes/sky/*.json`, with fading
by time of day, rotation with the sun, weather and OptiFine's blend modes. Shader packs that draw their own sky win.

Other OptiFine features in resource packs:
- **Emissive textures:** with `optifine/emissive.properties` (`suffix.emissive=_e`), `textures/block/<name>_e.png` is
  drawn over the block at full brightness; mobs already used `<texture>_e`.
- **Connected Textures** (`optifine/ctm/**/*.properties`): the `ctm` (47 tiles), `ctm_compact`, `horizontal`,
  `vertical`, `top`, `random`, `repeat` and `fixed` methods, with `matchTiles`/`matchBlocks`, `connect`, `faces`,
  `heights`, `weights`, `symmetry` and `innerSeams`. The `overlay*`, `horizontal+vertical` and `vertical+horizontal`
  methods and rules limited by `biomes` are left out.
- **Natural Textures** (`optifine/natural.properties`): rotation and mirroring per block.
- **Random Entities** (`optifine/random/entity/...`): `<texture>2.png`, `3`... per mob, with the `textures`/`skins`,
  `weights`, `name`, `heights`, `baby` and `health` rules (rules with other tests, such as `biomes`, are skipped).
- **Custom Entity Models** (`optifine/cem/*.jem`): reloaded with the resource packs, without a restart, and for baby
  models too (`zombie_baby.jem`), as Fresh Animations uses them.

Mod keys (`input::register_key`) show in Controls > Key Binds, in a section per mod, with a reset button. Settings
pages lay buttons and sliders out in two columns. Translated name and description in the manifest:

```toml
[translations.pt_br]
name = "..."
description = "..."
```

Example: `horizon/mods/pojarush-3dskinlayers`.

Molang expressions in animations still count as 0; numeric values, `lerp_mode` `catmullrom`, `loop` and
`animation_length` work. Full example: `horizon/mods/pojarush-geckodemo` (key J).

OptiFine CEM: resource packs with `assets/minecraft/optifine/cem/<entity>.jem` (and `.jpm`) replace or add parts of
the mobs' models and animate them with expressions (`cem_animation.txt`: functions, `var.*`, `this`/`part`/ids,
`limb_swing`, `head_pitch`, `health`, `is_on_ground`...). The jem's textures must be in `textures/entity/`.

Full examples in this repository:

- `horizon/mods/pojarush-zoom`: its own key, the mouse wheel, zoom, a settings page and saved settings.
- `horizon/mods/pojarush-autototem`: reads the inventory, puts a totem in the off hand when the last one is used and
  shows a totem counter above the XP level.
- `horizon/mods/keystrokes-port`: a port of Horizon's official Keystrokes 3.0.1 (MIT) to the Pojarush API.
- `horizon/mods/pojarush-fps`: text in Minecraft's font on the HUD, enlarged with `render::set_hud_scale` (a function
  of Pojarush's Horizon), adjustable size and position and saved settings. With it installed, the launcher turns off
  Horizon's own overlay.
- `horizon/mods/pojarush-modmenu`: a Mod Menu like Fabric's (key M or the small icon on the title screen and in the
  pause menu): a list with icon, name and description on the left, details, links and settings of the chosen mod on
  the right.
- `horizon/mods/pojarush-shaders`: Iris's shader shortcuts (O opens the packs, K toggles, R reloads) and the
  "Shader Packs..." button in Video Settings, which opens the Iris-style packs screen.
- `horizon/mods/acvcape`: a port of Acv Cape (Java, Bedrock and Dungeons II capes, favorites, No Cape). The capes'
  textures are not in the repository: `horizon/mods/acvcape/fetch-capes.sh ACV-Cape-Fabric-<version>.jar` copies them
  from the jar.
- `horizon/mods/combat-hitboxes`: a port of Combat Hitboxes (Apache-2.0).
- `horizon/mods/pojarush-3dskinlayers`: an unofficial test port of 3D Skin Layers, by tr7zw (all credit to the
  original author; do not redistribute): the skin's second layer as 3D voxels.
- `horizon/crates/horizon-mod-sdk/examples/`: `welcome`, `coords-hud`, `beacons`, `ruby-content`, `predator-missile`.
- The full API contract: `horizon/crates/horizon-mod-sdk/wit/horizon.wit`.

More mods, and how to publish your own: https://github.com/WandersTest3/Pojarush_Test3-Mods

## 4. Install and turn on

1. Copy the `.hmod` to `Pojarush_Test3/horizon/game/horizon-mods/`, or use Packs > Mods > Import file, or install
   it from the Pojarush Mods tab of the Packs page.
2. Leave "Load mods at startup" on in the launcher.
3. In the game, open Options > Mods and turn the mod on. It starts off, and the list shows what each one may do.
4. Restart the game: blocks and items only load at startup.

A mod that doesn't load shows in red in the list, with the reason.

# Tutorial: making shaders

The game takes two formats in the `shaderpacks/` folder:

| Format | File | State |
|---|---|---|
| Horizon | `.hshader` (WGSL) | Checked before it runs; the recommended format |
| OptiFine/Iris | `.zip` or a folder with `shaders/` | Starling 0.7.8's translator with fixes; heavy packs may have visual glitches |

To check an OptiFine/Iris pack on a PC (translation and pipeline creation; it doesn't show the image), put the
`.zip` files in a folder and run, inside `horizon/`:

```
HORIZON_TEST_SHADER_PACKS=/path/to/packs cargo test -p horizon-shaders --release --test real_packs -- --ignored --nocapture
HORIZON_TEST_SHADER_PACKS=/path/to/packs cargo test -p horizon-client-render --release --lib real_packs_build -- --ignored --nocapture
```

The second needs a Vulkan GPU (Mesa's lavapipe works) and uses a phone's limit of 26 varyings; `ONLY=name` tests only
the packs with that name. `HORIZON_SHADER_DUMP=folder` writes the GLSL that goes to naga.

## 1. Tool

```sh
cd Pojarush_Test3/horizon
cargo install --path crates/horizon-shader-api
```

## 2. Create, check and pack

```sh
horizon-shader new my_pack --template post
horizon-shader check my_pack
horizon-shader pack my_pack
```

The templates are `post`, `sky` and `material`. `check` shows each stage with its cost, or each error with file, line
and column. `pack` makes `my_pack-0.1.0.hshader`.

A pack is a folder with `horizon-shaders.toml` and WGSL files:

```toml
schema = 1
id = "my_pack"
name = "My Pack"
version = "0.1.0"
api = "0.1"

[[post]]
file = "post.wgsl"

[[options]]
id = "saturation"
name = "Saturation"
type = "float"
default = 1.15
min = 0.0
max = 2.0
```

## 3. Stages

Each stage is a function your file defines. Horizon still does the culling and the world's mesh; the pack changes how
it looks.

| Stage | Function | What it does |
|---|---|---|
| `[sky]` | `fn sky(input: SkyInput) -> vec4<f32>` | The Overworld sky's color in each direction; sun, moon, stars and clouds go on top |
| `[material]` | `fn material(input: MaterialInput) -> Surface` | The lit color of each point of the blocks, before the fog |
| `[[post]]` | `fn post(input: PostInput) -> vec4<f32>` | A full-screen pass over the world's image (up to 8, in order), under the hand and the HUD |

Every stage reads `frame`: the camera's matrices and position, the sun's and the moon's direction, the time of day,
the sky and fog colors, rain and the dimension. `post` passes also read `scene_color`, `scene_depth`, `previous` and
the functions `depth_at(uv)` and `view_position(uv, depth)`.

Each option of the manifest becomes a WGSL `override` with the same name:

```wgsl
override saturation: f32 = 1.15;
```

Rules `check` enforces: the files declare no `@group`/`@binding`, no entry points and no names starting with
`horizon_`. The limit is 20,000 expressions and 64 texture reads per stage.

## 4. Install and turn on

1. Copy the pack's folder or the `.hshader` to `Pojarush_Test3/horizon/game/shaderpacks/`, or use Packs > Shaders >
   Import file.
2. Leave "Load shaders at startup" on in the launcher.
3. In the game: Options > Horizon Settings > Shader Packs. One pack runs at a time; tapping it again turns it off.
4. OptiFine/Iris packs: with the pack chosen, the "Shader Options..." button at the top of the list opens the pack's
   own screens, profiles and sliders (`shaders.properties` and `lang/`). The values are kept in
   `shaderpacks/<pack>.txt`, as in Iris, and the pack recompiles when you leave the page with changes.

While the pack runs, the game rereads its files twice a second: on save, it is checked and reloaded. If the new
version fails, the chat shows where and the last good version keeps drawing.

Full guide and example (`golden_hour`): `horizon/docs/shaders.md` and `horizon/crates/horizon-shader-api/packs/`.

# Building the launcher

```sh
ANDROID_NDK_HOME=$ANDROID_HOME/ndk/25.2.9519653 ./horizon/android/build-android.sh
./gradlew :Pojarush:assembleRelease
```

Gradle makes `Pojarush-arm64-v8a-release.apk` and `Pojarush-armeabi-v7a-release.apk` in
`Pojarush/build/outputs/apk/release/`. Each APK's `versionCode` is `pojarush_version_code` × 10, plus 2 for the 64-bit
one and 1 for the 32-bit one; raise `pojarush_version_code` in `Pojarush/gradle.properties` for every version.

Signing reads `pojarush.jks` with `.store_password.txt`, `.key_password.txt` and `.key_alias.txt` (or
`STORE_PASSWORD`, `KEY_PASSWORD`, `KEY_ALIAS`); see `SIGNING.md`. Microsoft sign-in needs an Azure OAuth client ID in
`.oauth_client_id.txt` or `OAUTH_CLIENT_ID`; without it the app plays offline.

## Layout

| Path | What it is |
|---|---|
| `Pojarush/` | The Android app (Kotlin, Compose) |
| `Pojarush/src/main/jniLibs/<abi>/` | `libhorizon_android.so` (the game), `libhorizon_prepare.so`, `libpojajava.so`, `libc++_shared.so` |
| `Pojarush/src/main/assets/horizon/` | `research-lock.json` and the exporters used for preparation |
| `Pojarush/src/main/assets/controls/` | Default controls |
| `runtimes/jre-25/` | Java 25 for arm64 and 32-bit arm (from Zalith Launcher 2), downloaded by the app on the first preparation |
| `horizon/` | Horizon's code with the Android port (`crates/horizon-android`, `horizon-client/src/android.rs`) |
| `horizon/mods/pojarush-zoom/` | Pojarush's zoom mod |
| `horizon/mods/pojarush-autototem/` | Pojarush's Auto Totem mod |
| `horizon/mods/keystrokes-port/` | Keystrokes, ported from Horizon's official mod |
| `horizon/mods/pojarush-fps/` | Pojarush's FPS mod |
| `horizon/mods/pojarush-modmenu/` | Pojarush's Mod Menu |
| `horizon/mods/pojarush-shaders/` | Pojarush's shaders mod (Iris shortcuts) |
| `horizon/mods/pojarush-geckodemo/` | An example of the GeckoLib-style model API |
| `horizon/mods/pojarush-3dskinlayers/` | A test port of 3D Skin Layers (tr7zw) |
| `horizon/mods/combat-hitboxes/` | A port of Combat Hitboxes |
| `horizon/mods/acvcape/` | A port of Acv Cape |
