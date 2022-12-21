#![recursion_limit = "512"]
// Linter settings.
#![warn(missing_debug_implementations, bare_trait_objects, rust_2018_idioms)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::non_ascii_literal,
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::let_unit_value, // emitted by `yew::html!` proc macro
    clippy::unused_unit // emitted by `wasm_bindgen` proc macro
)]

use wasm_bindgen::{prelude::*, UnwrapThrowExt};
use yew::Renderer;

mod components;
pub mod js;
mod layout;
pub mod pages;
pub mod poll;
mod rng;
#[cfg(feature = "testing")]
pub mod testing;
mod utils;

use self::{js::JsAppProperties, pages::App};

#[wasm_bindgen(js_name = runApp)]
pub fn run_app(props: JsAppProperties) {
    let window = web_sys::window().expect_throw("no Window");
    let document = window.document().expect_throw("no Document");
    let element = document
        .query_selector("#app-root")
        .expect_throw("cannot get app root node")
        .expect_throw("cannot unwrap body node");

    Renderer::<App>::with_root_and_props(element, props.into()).render();
}
