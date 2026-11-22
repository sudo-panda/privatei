//! Private-I core: data, storage, calculations and AI calls.
//!
//! Pure Rust, no UI and no Tauri, so the same library can serve the Mac app
//! today and an Android or iPhone app later. Every number the UI shows is
//! computed here.

pub mod ai;
pub mod app;
pub mod backup;
pub mod dates;
pub mod food;
pub mod fooddb;
#[cfg(feature = "import")]
pub mod importers;
pub mod meds;
pub mod model;
pub mod nutrients;
pub mod store;
pub mod training;

pub use app::Core;
