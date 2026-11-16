//! Android: Pojarush_Test3's HorizonActivity (a GameActivity) loads this library and
//! android-activity starts `android_main` on the game's thread. The command line comes in
//! `HORIZON_ANDROID_ARGS` (fields split by U+001F), the working directory in
//! `POJARUSH_GAME_DIR`. The touch overlay talks to the game through `HorizonBridge`'s natives:
//! agent-control command lines (docs/horizon-ctl-commands.md) and look deltas, run on the game
//! thread once per frame (`game/android_bridge.rs`).

use android_activity::AndroidApp;
use jni::{
    JNIEnv,
    objects::{JClass, JString},
    sys::{JNI_FALSE, JNI_TRUE, jboolean, jfloat, jint},
};
use std::{
    collections::VecDeque,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

static APP: OnceLock<AndroidApp> = OnceLock::new();
static INPUT: Mutex<VecDeque<Input>> = Mutex::new(VecDeque::new());
static CAPTURED: AtomicBool = AtomicBool::new(false);
static READY: AtomicBool = AtomicBool::new(false);
static MUTED: AtomicBool = AtomicBool::new(false);
static TYPING: AtomicBool = AtomicBool::new(false);

/// Whether a text field of the game has keyboard focus: the overlay's buttons stop typing.
pub(crate) fn set_typing(on: bool) {
    TYPING.store(on, Ordering::Relaxed);
}

static FIELD: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Where the focused field ends, as a fraction of the window's height from
/// its top: the launcher lifts the game just enough to show it.
pub(crate) fn set_field(fraction: f32) {
    FIELD.store(fraction.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
}

pub(crate) fn muted() -> bool {
    MUTED.load(Ordering::Relaxed)
}

pub(crate) fn ready() -> bool {
    READY.load(Ordering::Relaxed)
}

pub(crate) fn set_ready() {
    READY.store(true, Ordering::Relaxed);
}

/// What the overlay sent since the last frame.
pub(crate) enum Input {
    /// An agent-control command line (`key w down`, `mouse left click`, `hotbar 3`...).
    Command(String),
    /// A drag on the game surface, in pixels, as a grabbed mouse's motion.
    Look(f64, f64),
    /// Mouse wheel lines (the hotbar while playing, scrolling in menus).
    Wheel(f32),
    /// The overlay's virtual mouse moved to this point of the view, in pixels.
    Cursor(f64, f64),
    /// A virtual mouse button (0 left, 1 right, 2 middle) at the cursor.
    Button(u8, bool),
}

static SCALE_X: AtomicU64 = AtomicU64::new(0);
static SCALE_Y: AtomicU64 = AtomicU64::new(0);
static SURFACE: AtomicU64 = AtomicU64::new(0);

pub(crate) fn set_surface(width: u32, height: u32) {
    SURFACE.store(u64::from(width) << 32 | u64::from(height), Ordering::Relaxed);
}

static HUD_DRAG_AREAS: std::sync::Mutex<Vec<(usize, [f32; 4])>> = std::sync::Mutex::new(Vec::new());
static HUD_DRAG: std::sync::Mutex<Option<(usize, f64, f64)>> = std::sync::Mutex::new(None);
static HUD_DRAG_MOVES: std::sync::Mutex<Vec<(usize, f32, f32, bool)>> = std::sync::Mutex::new(Vec::new());

pub(crate) fn set_hud_drag_areas(areas: Vec<(usize, [f32; 4])>) {
    if let Ok(mut a) = HUD_DRAG_AREAS.lock() {
        *a = areas;
    }
}

pub(crate) fn take_hud_drag_moves() -> Vec<(usize, f32, f32, bool)> {
    HUD_DRAG_MOVES.lock().map(|mut m| std::mem::take(&mut *m)).unwrap_or_default()
}

fn gui_point(x: f64, y: f64) -> Option<(f64, f64)> {
    let packed = SURFACE.load(Ordering::Relaxed);
    let (w, h) = ((packed >> 32) as u32, packed as u32);
    if w == 0 || h == 0 {
        return None;
    }
    let k = f64::from(horizon_client::hud::HudBatch::new([w, h]).scale);
    let (sx, sy) = scale();
    Some((x * sx / k, y * sy / k))
}

fn hud_drag_start(x: f64, y: f64) -> bool {
    let Some((gx, gy)) = gui_point(x, y) else { return false };
    let hit = HUD_DRAG_AREAS.lock().ok().and_then(|a| {
        a.iter().rev().find(|(_, r)| {
            let (rx, ry, rw, rh) = (f64::from(r[0]), f64::from(r[1]), f64::from(r[2]), f64::from(r[3]));
            gx >= rx - 4.0 && gx <= rx + rw + 4.0 && gy >= ry - 4.0 && gy <= ry + rh + 4.0
        }).map(|(i, _)| *i)
    });
    let Some(i) = hit else { return false };
    if let Ok(mut d) = HUD_DRAG.lock() {
        *d = Some((i, gx, gy));
    }
    true
}

fn hud_drag_move(x: f64, y: f64, done: bool) {
    let Ok(mut d) = HUD_DRAG.lock() else { return };
    let Some((i, lx, ly)) = *d else { return };
    let (gx, gy) = gui_point(x, y).unwrap_or((lx, ly));
    *d = if done { None } else { Some((i, gx, gy)) };
    if let Ok(mut m) = HUD_DRAG_MOVES.lock() {
        m.push((i, (gx - lx) as f32, (gy - ly) as f32, done));
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_wanderstest3_pojarush_horizon_HorizonBridge_nativeHudDragStart<
    'local,
>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    x: jfloat,
    y: jfloat,
) -> jboolean {
    if hud_drag_start(f64::from(x), f64::from(y)) {
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_wanderstest3_pojarush_horizon_HorizonBridge_nativeHudDragMove<
    'local,
>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    x: jfloat,
    y: jfloat,
    done: jboolean,
) {
    hud_drag_move(f64::from(x), f64::from(y), done != 0);
}

/// The hotbar slot (0-8) under a point of the view, as the HUD lays it out.
fn hotbar_slot(x: f64, y: f64) -> i32 {
    let packed = SURFACE.load(Ordering::Relaxed);
    let (w, h) = ((packed >> 32) as u32, packed as u32);
    if w == 0 || h == 0 {
        return -1;
    }
    let batch = horizon_client::hud::HudBatch::new([w, h]);
    let k = f64::from(batch.scale);
    let (sx, sy) = scale();
    let (gx, gy) = (x * sx / k, y * sy / k);
    let left = (f64::from(batch.width) / 2.0).floor() - 91.0;
    let top = f64::from(batch.height) - 22.0;
    if gy < top || gx < left || gx >= left + 182.0 {
        return -1;
    }
    (((gx - left - 1.0) / 20.0).floor() as i32).clamp(0, 8)
}

/// Surface pixels per view pixel on each axis: the launcher's resolution, which may
/// stretch the picture to the screen.
pub(crate) fn scale() -> (f64, f64) {
    let read = |a: &AtomicU64| {
        let bits = a.load(Ordering::Relaxed);
        (bits != 0).then(|| f64::from_bits(bits))
    };
    if let (Some(x), Some(y)) = (read(&SCALE_X), read(&SCALE_Y)) {
        return (x, y);
    }
    let s = std::env::var("POJARUSH_RENDER_SCALE")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| (0.05..=4.00).contains(v))
        .unwrap_or(1.00);
    (s, s)
}

fn set_scale(x: f64, y: f64) {
    if x.is_finite() && y.is_finite() && x > 0.0 && y > 0.0 {
        SCALE_X.store(x.to_bits(), Ordering::Relaxed);
        SCALE_Y.store(y.to_bits(), Ordering::Relaxed);
    }
}

/// The activity android-activity handed to `android_main`.
pub(crate) fn app() -> AndroidApp {
    APP.get().expect("android_main sets the app first").clone()
}

pub(crate) fn args() -> Vec<String> {
    std::env::var("HORIZON_ANDROID_ARGS")
        .map(|line| {
            line.split('\u{1f}')
                .filter(|a| !a.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn take_input() -> Vec<Input> {
    INPUT
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .drain(..)
        .collect()
}

/// Whether the game holds the pointer (playing, no screen open): the overlay shows its
/// controls and turns drags into looking.
pub(crate) fn set_captured(on: bool) {
    CAPTURED.store(on, Ordering::Relaxed);
}

/// The exit code for the launcher, in `POJARUSH_EXIT_FILE`.
pub(crate) fn finished(code: i32) {
    if let Some(path) = std::env::var_os("POJARUSH_EXIT_FILE") {
        let _ = std::fs::write(path, code.to_string());
    }
}

fn push(input: Input) {
    let mut queue = INPUT
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    match (&input, queue.back_mut()) {
        (Input::Look(x, y), Some(Input::Look(px, py))) => {
            *px += x;
            *py += y;
            return;
        }
        (Input::Cursor(x, y), Some(Input::Cursor(px, py))) => {
            *px = *x;
            *py = *y;
            return;
        }
        _ => {}
    }
    queue.push_back(input);
}

#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    let _ = APP.set(app);
    if let Some(dir) = std::env::var_os("POJARUSH_GAME_DIR").filter(|d| !d.is_empty()) {
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::env::set_current_dir(&dir);
    }
    super::main();
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_wanderstest3_pojarush_horizon_HorizonBridge_nativeCommand<
    'local,
>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    line: JString<'local>,
) {
    if let Ok(line) = env.get_string(&line) {
        let line: String = line.into();
        if !line.trim().is_empty() {
            push(Input::Command(line));
        }
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_wanderstest3_pojarush_horizon_HorizonBridge_nativeLook<
    'local,
>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    dx: jfloat,
    dy: jfloat,
) {
    push(Input::Look(f64::from(dx), f64::from(dy)));
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_wanderstest3_pojarush_horizon_HorizonBridge_nativeWheel<
    'local,
>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    lines: jfloat,
) {
    push(Input::Wheel(lines));
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_wanderstest3_pojarush_horizon_HorizonBridge_nativeCursor<
    'local,
>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    x: jfloat,
    y: jfloat,
) {
    push(Input::Cursor(f64::from(x), f64::from(y)));
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_wanderstest3_pojarush_horizon_HorizonBridge_nativeButton<
    'local,
>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    button: jint,
    down: jboolean,
) {
    push(Input::Button(button.clamp(0, 2) as u8, down != JNI_FALSE));
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_wanderstest3_pojarush_horizon_HorizonBridge_nativeSetScale<
    'local,
>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    x: jfloat,
    y: jfloat,
) {
    set_scale(f64::from(x), f64::from(y));
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_wanderstest3_pojarush_horizon_HorizonBridge_nativeHotbarSlot<
    'local,
>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    x: jfloat,
    y: jfloat,
) -> jint {
    hotbar_slot(f64::from(x), f64::from(y))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_wanderstest3_pojarush_horizon_HorizonBridge_nativeIsCaptured<
    'local,
>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
) -> jboolean {
    if CAPTURED.load(Ordering::Relaxed) {
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_wanderstest3_pojarush_horizon_HorizonBridge_nativeIsReady<
    'local,
>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
) -> jboolean {
    if READY.load(Ordering::Relaxed) {
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_wanderstest3_pojarush_horizon_HorizonBridge_nativeSetMuted<
    'local,
>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    muted: jboolean,
) {
    MUTED.store(muted != 0, Ordering::Relaxed);
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_wanderstest3_pojarush_horizon_HorizonBridge_nativeFieldBottom<
    'local,
>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
) -> jni::sys::jfloat {
    f32::from_bits(FIELD.load(Ordering::Relaxed))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_wanderstest3_pojarush_horizon_HorizonBridge_nativeIsTyping<
    'local,
>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
) -> jboolean {
    if TYPING.load(Ordering::Relaxed) {
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}
