//! Pojarush_Test3's touch overlay (horizon_client::pojarush): its command lines, look deltas
//! and virtual mouse run here once per loop iteration, and touches on the game surface act as a
//! mouse while no world holds the pointer. Starling's own touch controls stay hidden.
#![cfg(all(target_os = "android", feature = "pojarush"))]
use super::{Game, Screen, event_loop::EventLoopCtl, input_trace::InputOrigin};
use horizon_client::pojarush::{self as bridge, Input};
use std::{cell::Cell, time::Instant};
use winit::{
    dpi::PhysicalPosition,
    event::{DeviceId, ElementState, MouseButton, MouseScrollDelta, Touch, TouchPhase, WindowEvent},
    window::WindowId,
};

#[derive(Clone, Copy, Default)]
struct TouchPointer {
    primary: Option<u64>,
    scroll: Option<(u64, f64)>,
}

thread_local! {
    static POINTER: Cell<TouchPointer> = Cell::new(TouchPointer::default());
    static SMOOTH: Cell<(Option<Instant>, u32)> = const { Cell::new((None, 0)) };
    static FIELD: Cell<Option<f32>> = const { Cell::new(None) };
}

const READY_FRAMES: u32 = 12;
const READY_GAP_MS: u128 = 90;

impl Game<'_> {
    pub(super) fn pojarush_frame(&mut self, event_loop: &dyn EventLoopCtl) {
        bridge::set_captured(self.captured && self.screen == Screen::Playing);
        let context = self.text_context();
        let typing = context.is_some();
        bridge::set_typing(typing);
        let drawn = match horizon_client_render::ui::take_field() {
            Some(b) if typing => {
                FIELD.set(Some(b));
                Some(b)
            }
            _ if typing => FIELD.get(),
            _ => {
                FIELD.set(None);
                None
            }
        };
        let (gui_w, gui_h) = self.gui_size();
        let bottom = match context {
            None => 0.0,
            Some("chat") => gui_h,
            Some("sign") => drawn.unwrap_or(135.0),
            Some("book") => drawn.unwrap_or(170.0),
            Some(_) => drawn.unwrap_or(gui_h),
        };
        bridge::set_field(if gui_h > 0.0 { bottom / gui_h } else { 1.0 });
        if let Some(r) = self.renderer.as_ref()
            && gui_w > 0.0
        {
            bridge::set_gui(gui_w, gui_h, r.config.width as f32 / gui_w);
        }
        let master = if bridge::muted() { 0.0 } else { self.settings.master_volume };
        if self.audio.master != master {
            self.audio.master = master;
        }
        if !bridge::ready() {
            let loaded = self.rest_loader.is_none() && self.renderer.is_some();
            let now = Instant::now();
            let (last, count) = SMOOTH.get();
            let smooth = last.is_some_and(|l| now.duration_since(l).as_millis() < READY_GAP_MS);
            let count = if loaded && smooth { count + 1 } else { 0 };
            SMOOTH.set((Some(now), count));
            if count >= READY_FRAMES {
                bridge::set_ready();
            }
        }
        if bridge::muted() && bridge::ready() {
            let until = Instant::now() + std::time::Duration::from_millis(1000);
            while bridge::muted() && Instant::now() < until {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        }
        let Some(id) = self.window.as_ref().map(|w| w.id()) else {
            return;
        };
        let device_id = DeviceId::dummy();
        for input in bridge::take_input() {
            match input {
                Input::Look(x, y) => {
                    if self.captured && self.screen == Screen::Playing {
                        self.grabbed_mouse_motion(x, y);
                        self.last_input = Instant::now();
                    }
                }
                Input::Wheel(lines) => self.handle_window_event(
                    event_loop,
                    id,
                    WindowEvent::MouseWheel {
                        device_id,
                        delta: MouseScrollDelta::LineDelta(0.0, lines),
                        phase: TouchPhase::Moved,
                    },
                    InputOrigin::User,
                ),
                Input::Cursor(x, y) => {
                    let (sx, sy) = bridge::scale();
                    self.handle_window_event(
                        event_loop,
                        id,
                        WindowEvent::CursorMoved {
                            device_id,
                            position: PhysicalPosition::new(x * sx, y * sy),
                        },
                        InputOrigin::User,
                    );
                }
                Input::Button(button, down) => {
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
                    self.handle_window_event(
                        event_loop,
                        id,
                        WindowEvent::MouseInput {
                            device_id,
                            state,
                            button,
                        },
                        InputOrigin::User,
                    );
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

    /// A touch on the game surface: the mouse in menus, nothing while playing (the overlay
    /// has the fingers then). Always taken.
    pub(super) fn pojarush_touch(&mut self, event_loop: &dyn EventLoopCtl, id: WindowId, touch: &Touch) {
        if self.captured && self.screen == Screen::Playing {
            return;
        }
        let mut pointer = POINTER.get();
        let device_id = touch.device_id;
        let (sx, sy) = bridge::scale();
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
                    events.push(WindowEvent::MouseWheel {
                        device_id,
                        delta: MouseScrollDelta::PixelDelta(PhysicalPosition::new(
                            0.0,
                            position.y - last,
                        )),
                        phase: TouchPhase::Moved,
                    });
                } else if pointer.primary == Some(touch.id) && pointer.scroll.is_none() {
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
            self.handle_window_event(event_loop, id, event, InputOrigin::User);
        }
    }
}
