pub mod config;
pub mod events;
pub mod hud;
pub mod i18n;
pub mod keys;
pub mod scheduler;
pub mod settings;
pub mod text;

pub use horizon_mod_sdk;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
