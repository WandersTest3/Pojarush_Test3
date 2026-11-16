use crate::keys::Keys;
use horizon_mod_sdk::input::{self, Event as Raw};

#[derive(Clone, Debug)]
pub enum Event {
    KeyDown(String),
    KeyUp(String),
    Wheel(f32),
    UseStart { item: String, slot: u32 },
    UseStop { item: String, slot: u32, ticks: u32 },
}

pub fn poll(keys: &mut Keys) -> Vec<Event> {
    input::poll()
        .into_iter()
        .map(|e| match e {
            Raw::KeyDown(k) => {
                keys.pressed(&k);
                Event::KeyDown(k)
            }
            Raw::KeyUp(k) => {
                keys.released(&k);
                Event::KeyUp(k)
            }
            Raw::Wheel(s) => Event::Wheel(s),
            Raw::UseStart(u) => Event::UseStart { item: u.item.id, slot: u.item.slot },
            Raw::UseStop(u) => Event::UseStop { item: u.item.id, slot: u.item.slot, ticks: u.ticks },
        })
        .collect()
}

pub fn holding(id: &str) -> bool {
    input::held().is_some_and(|h| h.id == id)
}
