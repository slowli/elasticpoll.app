#![recursion_limit = "512"]
// Linter settings.
#![warn(missing_debug_implementations, bare_trait_objects)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::non_ascii_literal,
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::needless_pass_by_value
)]

use wasm_bindgen::{prelude::*, UnwrapThrowExt};

mod components;
mod js;
mod layout;
mod pages;
mod poll;
mod rng;
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

    yew::start_app_with_props_in_element::<App>(element, props.into());
}
