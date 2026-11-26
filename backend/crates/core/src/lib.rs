//! # PriEco Core
//!
//! Global constants, objects and functions used by all PriEco crates.
//!
//! * [constants] - Paths and sizes.
//! * [globals] - Shared objects: index, HTTP clients, models, analytics, colors and icons.
//! * [helpers] - Small file, URL and hashing functions.
//! * [set_up] - Node config and first run set up.

pub mod constants;
pub mod globals;
pub mod helpers;
pub mod set_up;

pub use constants::*;
pub use globals::*;
pub use helpers::*;
