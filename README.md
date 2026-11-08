# Pojarush_Test3

Android launcher with a single game: **Horizon**, Minecraft: Java Edition 26.3 rewritten in Rust, running
natively on the phone (no JVM while playing).

- Horizon: https://github.com/horizon-rust/horizon (MIT), source in `horizon/`

## Layout

| Path | What |
|---|---|
| `Pojarush/` | The Android app (Kotlin, Compose) |
| `Pojarush/src/main/java/com/wanderstest3/pojarush/MainActivity.kt` | The single screen: player, Horizon 26.3, logs |
| `.../Accounts.kt`, `MicrosoftAuth.kt` | Offline player name; Microsoft sign-in when the build has an OAuth client ID |
| `.../JavaRuntime.kt` | Unpacks the bundled Java 25 used only to prepare the game files |
| `.../horizon/HorizonManager.kt` | Downloads 26.3 from Mojang (SHA-1 checked), runs `horizon-prepare` |
| `.../horizon/HorizonActivity.kt`, `HorizonOverlay.kt`, `HorizonBridge.kt` | The game (`GameActivity`, process `:horizon`) and its touch controls |
| `Pojarush/src/main/jniLibs/arm64-v8a/` | `libhorizon_android.so` (the game), `libhorizon_prepare.so`, `libpojajava.so` (Java launcher for the exporters), `libc++_shared.so` (needed by Java 25) |
| `Pojarush/src/main/assets/horizon/` | `research-lock.json`, exporter sources and precompiled exporter classes |
| `Pojarush/src/main/assets/runtimes/jre-25/` | Java 25 for arm64 and 32-bit arm (from Zalith Launcher 2) |
| `horizon/` | Horizon's source with the Android port (`crates/horizon-android`, `horizon-client/src/android.rs`, `game/android_bridge.rs`, `android/`) |

## Build

```sh
ANDROID_NDK_HOME=$ANDROID_HOME/ndk/25.2.9519653 ./horizon/android/build-android.sh
./gradlew :Pojarush:assembleRelease
```

Release signing reads `pojarush.jks` with `.store_password.txt`, `.key_password.txt` and `.key_alias.txt`
(or `STORE_PASSWORD`, `KEY_PASSWORD`, `KEY_ALIAS`). Microsoft sign-in needs an Azure OAuth client ID with
Minecraft API access in `.oauth_client_id.txt` or `OAUTH_CLIENT_ID`; without it the app plays offline.

## Use

1. Choose a player name or sign in with Microsoft.
2. Optional: allow access to the internal storage; worlds, options, packs, layouts and logs then live in
   `Pojarush_Test3/` at its root (otherwise in the app's folder).
3. Prepare game files: about 330 MB download (+380 MB with sounds), then several minutes of Java exporters.
4. Play (the button at the bottom right).

Controls: the built-in Pojarush layout and any Pojarush (`pojarush`), Zalith (`editorVersion`),
Fold Craft Launcher (`viewGroups`) or PojavLauncher (`mControlDataList`) layout imported from a `.json` file or
put in `Pojarush_Test3/controls/`.
In game: drag to look, tap to use, hold to break, tap the hotbar to select. In menus: direct touch, or a
virtual mouse with an optional custom image or animated GIF (Mouse page). The floating button (drag it to move
it; size, opacity, roundness, color and image in Controls) opens the controls, mouse and resolution settings
directly, with quick actions for the virtual mouse, keyboard, hiding the controls and the control editor. Edited layouts are saved as Pojarush `.json` files in
`Pojarush_Test3/controls/`. Back is Escape.

Not an official Minecraft product; no Mojang files are included. You must own Minecraft: Java Edition.

## 1.8.0

- Code mods (.hmod with WebAssembly) run again on arm64: the sandboxed mod runtime (wasmtime) from
  Starling (Vasysik/horizon-android, MIT) is ported into the game, with its tick, frame, key,
  mouse, chat, command, HUD panel, zoom and settings-screen hooks. 32-bit phones load content mods
  only (wasmtime has no 32-bit arm backend).
- Mods page: "Download" lists the mods, resource packs and shaders of playhorizon.dev (its
  registry index), checks each file's SHA-256 and installs it in the right folder, turned on.
  Native mods (.hnmod) are hidden: they only run in PC builds.
- Pojarush Zoom (`horizon/mods/pojarush-zoom`), a test mod installed and turned on once: press C
  to zoom in, press C again to zoom out.
- Starting the game: the launcher-styled loading screen stays up until the resources finished
  loading and the game draws smoothly, then the game shows at once.
- Controls editor: the top panel moves (hold and drag), exact sizes can be typed, "Mirror size"
  keeps width and height equal, layouts can be named when saved and renamed later, and the
  background has random particles instead of stripes. "Edit layout" is back in the in-game menu.
  HUD images moved to the Controls page, under the editor.

## 1.7.0

- 32-bit phones (armeabi-v7a): the APK now carries Horizon, horizon-prepare, the Java shim and
  Java 25 for 32-bit arm as well as arm64.
- Fixed the crash on GPUs with 1024 texture layers ("GPU supports 1024 texture layers; 1291
  required"): block textures are split into arrays of the GPU's real layer limit.
- GPU timestamp queries are off on Android: after a GPU reset (Adreno 610 logs) the destroyed
  readback buffer panicked the game. `HORIZON_GPU_TIMER=1` turns them back on.
- Slower start fixed: 1.6.0 made the GUI atlas wait for the GPU and lowered the background
  loader's priority; the atlas now waits only when it does not fit 4096×4096, and the priority
  change is reverted.
- LOD Distance goes up to 65536 chunks (about 1 million blocks), as a slider in game and in the
  launcher's Graphics page; phones are no longer capped at 128 chunks.
- The FPS counter is the default overlay on Android.
- Launcher: "Others" page (interface, wallpaper, floating button, HUD images, extras), menu
  background opacity, Minecraft-style font (Pwnage_Block's "Minecraft" font, CC BY-SA 3.0, with
  Portuguese accented letters added), long labels scroll instead of being cut, RAM and
  resolution apply as you type, the 67%/45% shortcuts are gone, "Open folder" opens ZArchiver
  when installed.
- Floating button: Pojarush_Test3 icon by default (23 dp, 60% opacity, round) with a colored
  border and 20 colors; opens Controls, Mouse and Graphics.
- HUD images: import a .zip or images and use them on buttons and joysticks (stretched to the
  control); the built-in "Bedrock Controls" layout uses the Bedrock UI image names. No Mojang
  images are bundled: import your own pack.
- The skin preview now uses a per-pixel depth buffer, so the cape is never cut by an arm or leg.

## 1.6.0

- Resource packs load again: Rust's file locks are not implemented on Android, so the pack cache's
  lock always failed with "Resource reload failed: lock() not supported". Locks now use the kernel's
  `flock` and carry on unlocked on filesystems without them. The game log (`latest.log`, archived as
  `YYYY-MM-DD-N.log.gz` like vanilla) and world session locks use the same path; logs and crash
  reports are also copied to `Pojarush_Test3/logs` and `Pojarush_Test3/crash-reports`.
- Fixed a 1.5.1 regression that broke every resource pack (including the offline skin pack): the
  check that a linked file reads back always failed on folders, and the copy fallback then refused
  to copy a folder, so the whole overlay failed.
- Fixed the crash on GPUs limited to 4096×4096 textures (PowerVR GE8300 and others: "the shared
  atlas is 4096x5120"): the GUI atlas is now packed for the GPU's real limit with a tighter skyline
  packer, and only if it still does not fit are the large GUI textures halved.
- `.hshader` packs written for newer Horizon builds (`[[lights]]`, `[shadow]`, per-stage `files`...)
  are no longer refused: what this Horizon draws (sky, material, post) runs and the chat says which
  sections are not supported yet. WebAssembly code mods (e.g. Compass Enhanced) still do not run in
  this Horizon version; content `.hmod` mods do.
- PojavLauncher control layouts (`mControlDataList`, dynamic positions, drawers, special buttons)
  can be imported.
- Controls: hide them while keeping them usable, 80% default opacity, double-tap a hotbar slot to swap
  hands (F), Bedrock-style drop (hold a hotbar slot to keep pressing Q, interval adjustable), auto
  sprint per joystick, and an option to stop taps outside the buttons from clicking.
- The floating button is customizable and opens the settings directly.
- Launcher: wallpaper (image or muted looping video), Minecraft-style FPS counter, Mouse before Mods in
  the menu, a drawn Play icon (some devices showed an emoji), crash reason and log sharing next to
  "The last game ended with code N".
- The game and the Minecraft download keep running in the background through a foreground service
  (with a notification), so switching apps no longer closes the game or freezes the preparation;
  a stuck preparation is now stopped together with its Java process instead of hanging forever.
- Faster input right after the title screen appears: the background world loader runs at a lower
  priority on Android.
- The skin preview's cape is drawn with depth and no longer cut by the arms or body.

## 1.5.1

- Fixed `.hmod` content mods and resource packs still not loading after 1.5.0's private-cache fix:
  the overlay symlink itself lived in private storage, but when it pointed at a pack or mod the
  player keeps in shared storage (so it stays visible from outside the app), reading through that
  link across the two filesystems could still silently fail. The overlay builder now checks a link
  actually reads back and falls back to a real copy when it doesn't.
- `.hmod` files are now found regardless of case (`Mod.HMOD`, not just `mod.hmod`).
- A connected physical mouse now moves the cursor and scrolls; only touch and a Bluetooth keyboard
  worked before.
- A first run now asks, in order: internal-storage permission, then to sign in, then to start the
  Minecraft download — you can keep setting things up while it downloads.
- Editing a layout you already saved updates that file instead of creating a new one each time, and
  accented names (e.g. "Pojarush (padrão)") no longer get mangled when saved.
- New look for the launcher's home screen.
- Clarified in the Mods page: classic OptiFine/Iris `.zip` shader packs are listed but not rendered
  yet — Horizon only runs its own `.hshader` format so far.

## 1.5.0

- Several accounts (offline, Microsoft, Ely.by) with a switcher; skin viewer with cape.
- Mods page: `.hmod` content mods, resource packs and shader packs. Resource packs can be turned on
  from the launcher (written to `options.txt`); a zip holding the pack inside one folder is extracted.
- Resource-pack and content-mod overlays are built in the app's private storage (`HORIZON_CACHE_DIR`):
  shared storage refuses the symlinks they are made of, which kept packs and mods from loading.
- A leftover game process is stopped before the game starts again (two games on one world).
- The logs export includes the latest crash reports.
- Graphics API (Auto / Vulkan / OpenGL) through `HORIZON_GRAPHICS_API`; resolution recommendations.

Not supported: `.hmod` mods with code (`mod.wasm`) and native `.hnmod` mods, which must be compiled
into the game itself.
