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
| `Pojarush/src/main/assets/runtimes/jre-25/` | Java 25 for arm64 |
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

Controls: the built-in Pojarush layout and any Pojarush (`pojarush`), Zalith (`editorVersion`) or
Fold Craft Launcher (`viewGroups`) layout imported from a `.json` file or put in `Pojarush_Test3/controls/`.
In game: drag to look, tap to use, hold to break, tap the hotbar to select. In menus: direct touch, or a
virtual mouse with an optional custom image or animated GIF (Mouse page). The ☰ handle (drag it to move it)
opens the quick menu: virtual mouse, keyboard, settings (controls, mouse, resolution) and the control editor,
also reachable from the launcher. Edited layouts are saved as Pojarush `.json` files in
`Pojarush_Test3/controls/`. Back is Escape.

Not an official Minecraft product; no Mojang files are included. You must own Minecraft: Java Edition.

## Changes in this build (not compiled or tested in game)

- Several accounts (offline, Microsoft, Ely.by) with a switcher; skin viewer with cape.
- Mods page: import and list `.hmod` mods, resource packs and shader packs into the game folder; `horizon-mods`, `resourcepacks`, `shaderpacks` are created automatically.
- Graphics API (Auto / Vulkan / OpenGL) through `HORIZON_GRAPHICS_API`; resolution recommendations.
- Logs now go to the app's private storage (shared storage has no file locking). `logging.rs` also tolerates missing locks; it needs `build-android.sh` to take effect.
