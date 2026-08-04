//! Wasm entrypoint.
//!
//! Everything this crate does is take over markup the server already rendered.
//! There is no client-side bootstrap, no empty `<div id="app">` waiting to be
//! filled -- the page is complete and readable before this module loads, and
//! hydration only attaches the interactivity.

use wasm_bindgen::prelude::wasm_bindgen;

/// Called by the module `cargo-leptos` generates and injects into the page.
#[wasm_bindgen]
pub fn hydrate() {
    // Turns a wasm panic from an opaque "unreachable executed" into a real
    // message and stack trace in the browser console. Cheap, and the difference
    // between a debuggable report and a shrug.
    console_error_panic_hook::set_once();

    leptos::mount::hydrate_body(app::App);
}
