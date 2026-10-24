//! Integration tests for the prieco_web binary crate.
//!
//! prieco_web has no lib target, so its `web` module tree is compiled straight into this
//! test crate. Every `crate::web::...` path used by the real code resolves to the same
//! modules here, nothing in `src/` has to change.
//!
//! The whole binary runs in a temp sandbox (see `support::init`).

#![allow(dead_code, unused_imports, unused_variables)]

#[path = "../../src/web/mod.rs"]
pub mod web;

mod support;

mod apis;
mod assets;
mod currency;
mod decentralized;
mod general;
mod goggles;
mod hand;
mod meaning;
mod pages;
mod rrf;
mod search;
mod settings;
mod spell_check;
