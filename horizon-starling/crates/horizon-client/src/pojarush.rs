//! Pojarush_Test3: its HorizonActivity (a GameActivity) loads this library. The command line
//! comes in `HORIZON_ANDROID_ARGS` (fields split by U+001F), the game folder in
//! `POJARUSH_GAME_DIR`; the launcher's touch overlay talks to the game through `HorizonBridge`'s
//! natives (horizon-android's android_lib.rs), run on the game thread once per frame
//! (game/starling/pojarush_bridge.rs).

use std::{
    collections::VecDeque,
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
    },
};

static INPUT: Mutex<VecDeque<Input>> = Mutex::new(VecDeque::new());
static CAPTURED: AtomicBool = AtomicBool::new(false);
static READY: AtomicBool = AtomicBool::new(false);
static MUTED: AtomicBool = AtomicBool::new(false);
static TYPING: AtomicBool = AtomicBool::new(false);
static FIELD: AtomicU32 = AtomicU32::new(0);
static SCALE_X: AtomicU64 = AtomicU64::new(0);
static SCALE_Y: AtomicU64 = AtomicU64::new(0);
static GUI: AtomicU64 = AtomicU64::new(0);
static GUI_SCALE: AtomicU32 = AtomicU32::new(0);

/// Whether this library runs inside Pojarush (the launcher set its game folder).
pub fn active() -> bool {
    std::env::var_os("POJARUSH_GAME_DIR").is_some_and(|d| !d.is_empty())
}

/// What the overlay sent since the last frame.
pub enum Input {
    /// An agent-control command line (`key w down`, `mouse left click`, `hotbar 3`...).
    Command(String),
    /// A drag on the game surface, in pixels, as a grabbed mouse's motion.
    Look(f64, f64),
    /// Mouse wheel lines.
    Wheel(f32),
    /// The overlay's virtual mouse moved to this point of the view, in pixels.
    Cursor(f64, f64),
    /// A virtual mouse button (0 left, 1 right, 2 middle) at the cursor.
    Button(u8, bool),
}

pub fn push(input: Input) {
    let mut queue = INPUT.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
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

pub fn take_input() -> Vec<Input> {
    INPUT
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .drain(..)
        .collect()
}

pub fn set_typing(on: bool) {
    TYPING.store(on, Ordering::Relaxed);
}

pub fn typing() -> bool {
    TYPING.load(Ordering::Relaxed)
}

/// Where the focused field ends, as a fraction of the window's height from its top.
pub fn set_field(fraction: f32) {
    FIELD.store(fraction.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
}

pub fn field() -> f32 {
    f32::from_bits(FIELD.load(Ordering::Relaxed))
}

pub fn set_captured(on: bool) {
    CAPTURED.store(on, Ordering::Relaxed);
}

pub fn captured() -> bool {
    CAPTURED.load(Ordering::Relaxed)
}

pub fn set_ready() {
    READY.store(true, Ordering::Relaxed);
}

pub fn ready() -> bool {
    READY.load(Ordering::Relaxed)
}

pub fn set_muted(on: bool) {
    MUTED.store(on, Ordering::Relaxed);
}

pub fn muted() -> bool {
    MUTED.load(Ordering::Relaxed)
}

/// The GUI's size in its own units and its pixels per unit, for `hotbar_slot`.
pub fn set_gui(width: f32, height: f32, scale: f32) {
    GUI.store(
        u64::from(width.to_bits()) << 32 | u64::from(height.to_bits()),
        Ordering::Relaxed,
    );
    GUI_SCALE.store(scale.to_bits(), Ordering::Relaxed);
}

/// The hotbar slot (0-8) under a point of the view, as the HUD lays it out.
pub fn hotbar_slot(x: f64, y: f64) -> i32 {
    let packed = GUI.load(Ordering::Relaxed);
    let (w, h) = (
        f64::from(f32::from_bits((packed >> 32) as u32)),
        f64::from(f32::from_bits(packed as u32)),
    );
    let k = f64::from(f32::from_bits(GUI_SCALE.load(Ordering::Relaxed)));
    if w <= 0.0 || h <= 0.0 || k <= 0.0 {
        return -1;
    }
    let (sx, sy) = scale();
    let (gx, gy) = (x * sx / k, y * sy / k);
    let left = (w / 2.0).floor() - 91.0;
    let top = h - 22.0;
    if gy < top || gx < left || gx >= left + 182.0 {
        return -1;
    }
    (((gx - left - 1.0) / 20.0).floor() as i32).clamp(0, 8)
}

/// Surface pixels per view pixel on each axis: the launcher's resolution.
pub fn scale() -> (f64, f64) {
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

pub fn set_scale(x: f64, y: f64) {
    if x.is_finite() && y.is_finite() && x > 0.0 && y > 0.0 {
        SCALE_X.store(x.to_bits(), Ordering::Relaxed);
        SCALE_Y.store(y.to_bits(), Ordering::Relaxed);
    }
}

/// The launcher's command line: `HORIZON_ANDROID_ARGS`, fields split by U+001F.
pub fn args() -> Vec<String> {
    std::env::var("HORIZON_ANDROID_ARGS")
        .map(|line| {
            line.split('\u{1f}')
                .filter(|a| !a.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// The exit code for the launcher, in `POJARUSH_EXIT_FILE`.
pub fn finished(code: i32) {
    if let Some(path) = std::env::var_os("POJARUSH_EXIT_FILE") {
        let _ = std::fs::write(path, code.to_string());
    }
}
