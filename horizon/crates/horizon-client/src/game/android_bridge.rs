//! Android input: the overlay's command lines and look deltas (android.rs), and touches on
//! the game surface as a mouse while no world holds the pointer (menus, inventories).
use super::{Game, Screen};
use crate::android::{self, Input};
use std::{cell::Cell, time::Instant};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalPosition,
    event::{DeviceId, ElementState, MouseButton, MouseScrollDelta, Touch, TouchPhase, WindowEvent},
    event_loop::ActiveEventLoop,
    window::WindowId,
};

/// The finger that acts as the mouse, and a second one that scrolls.
#[derive(Clone, Copy, Default)]
struct TouchPointer {
    primary: Option<u64>,
    scroll: Option<(u64, f64)>,
}

thread_local! {
    static POINTER: Cell<TouchPointer> = Cell::new(TouchPointer::default());
    static SMOOTH: Cell<(Option<Instant>, u32)> = const { Cell::new((None, 0)) };
}

const READY_FRAMES: u32 = 12;
const READY_GAP_MS: u128 = 90;

impl Game<'_> {
    pub(super) fn android_frame(&mut self, event_loop: &ActiveEventLoop) {
        android::set_captured(self.captured && self.screen == Screen::Playing);
        let master = if android::muted() { 0.0 } else { self.settings.master_volume };
        if self.audio.master != master {
            self.audio.master = master;
        }
        if !android::ready() {
            let loaded = self.rest_loader.is_none()
                && self.res.rest.get().is_some()
                && self.renderer.is_some();
            let now = Instant::now();
            let (last, count) = SMOOTH.get();
            let smooth = last.is_some_and(|l| now.duration_since(l).as_millis() < READY_GAP_MS);
            let count = if loaded && smooth { count + 1 } else { 0 };
            SMOOTH.set((Some(now), count));
            if count >= READY_FRAMES {
                android::set_ready();
            }
        }
        if android::muted() && android::ready() {
            let until = Instant::now() + std::time::Duration::from_millis(1000);
            while android::muted() && Instant::now() < until {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        }
        if let Some(size) = self.window.as_ref().map(|w| w.inner_size()) {
            android::set_surface(size.width, size.height);
        }
        for input in android::take_input() {
            match input {
                Input::Look(x, y) => {
                    if self.pending_save.is_none()
                        && self.captured
                        && self.screen == Screen::Playing
                    {
                        self.mouse.on_move(0.0, 0.0, x, y, true, true);
                        self.last_input = Instant::now();
                    }
                }
                Input::Wheel(lines) => {
                    if let Some(id) = self.window.as_ref().map(|w| w.id()) {
                        let device_id = DeviceId::dummy();
                        self.window_event(
                            event_loop,
                            id,
                            WindowEvent::MouseWheel {
                                device_id,
                                delta: MouseScrollDelta::LineDelta(0.0, lines),
                                phase: TouchPhase::Moved,
                            },
                        );
                    }
                }
                Input::Cursor(x, y) => {
                    if let Some(id) = self.window.as_ref().map(|w| w.id()) {
                        let (sx, sy) = android::scale();
                        self.window_event(
                            event_loop,
                            id,
                            WindowEvent::CursorMoved {
                                device_id: DeviceId::dummy(),
                                position: PhysicalPosition::new(x * sx, y * sy),
                            },
                        );
                    }
                }
                Input::Button(button, down) => {
                    if let Some(id) = self.window.as_ref().map(|w| w.id()) {
                        let button = match button {
                            1 => MouseButton::Right,
                            2 => MouseButton::Middle,
                            _ => MouseButton::Left,
                        };
                        let state = if down {
                            ElementState::Pressed
                        } else {
                            ElementState::Released
                        };
                        self.window_event(
                            event_loop,
                            id,
                            WindowEvent::MouseInput {
                                device_id: DeviceId::dummy(),
                                state,
                                button,
                            },
                        );
                    }
                    self.last_input = Instant::now();
                }
                Input::Command(line) => {
                    let (reply, _answer) = std::sync::mpsc::channel();
                    if let Err(e) = self.control_command(&line, &reply, event_loop) {
                        log::debug!("overlay command {line:?}: {e}");
                    }
                    self.last_input = Instant::now();
                }
            }
        }
    }

    pub(super) fn android_touch(&mut self, event_loop: &ActiveEventLoop, id: WindowId, touch: Touch) {
        if self.captured && self.screen == Screen::Playing {
            return;
        }
        let mut pointer = POINTER.get();
        let device_id = touch.device_id;
        let (sx, sy) = android::scale();
        let position = PhysicalPosition::new(touch.location.x * sx, touch.location.y * sy);
        let moved = WindowEvent::CursorMoved {
            device_id,
            position,
        };
        let button = |state| WindowEvent::MouseInput {
            device_id,
            state,
            button: MouseButton::Left,
        };
        let mut events = Vec::new();
        match touch.phase {
            TouchPhase::Started => {
                if pointer.primary.is_none() {
                    pointer.primary = Some(touch.id);
                    events.push(moved.clone());
                    events.push(button(ElementState::Pressed));
                } else if pointer.scroll.is_none() {
                    pointer.scroll = Some((touch.id, position.y));
                    events.push(button(ElementState::Released));
                }
            }
            TouchPhase::Moved => {
                if let Some((finger, last)) = pointer.scroll
                    && finger == touch.id
                {
                    pointer.scroll = Some((finger, position.y));
                    let delta = MouseScrollDelta::PixelDelta(PhysicalPosition::new(
                        0.0,
                        position.y - last,
                    ));
                    events.push(WindowEvent::MouseWheel {
                        device_id,
                        delta,
                        phase: TouchPhase::Moved,
                    });
                } else if pointer.primary == Some(touch.id)
                    && pointer.scroll.is_none()
                {
                    events.push(moved.clone());
                }
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                if pointer.scroll.is_some_and(|(f, _)| f == touch.id) {
                    pointer.scroll = None;
                } else if pointer.primary == Some(touch.id) {
                    pointer.primary = None;
                    if pointer.scroll.take().is_none() {
                        events.push(moved.clone());
                        events.push(button(ElementState::Released));
                    }
                }
            }
        }
        POINTER.set(pointer);
        for event in events {
            self.window_event(event_loop, id, event);
        }
    }
}
