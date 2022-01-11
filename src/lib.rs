#![recursion_limit = "512"]
// Linter settings.
#![warn(missing_debug_implementations, bare_trait_objects)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::non_ascii_literal,
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::needless_pass_by_value // FIXME: move to appropriate place
)]

use wasm_bindgen::{prelude::*, UnwrapThrowExt};
use yew::Callback;

mod components;
mod layout;
mod poll;
mod rng;
mod utils;

use self::components::{App, AppProperties, ExportedData};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = AppProperties)]
    pub type JsAppProperties;

    #[wasm_bindgen(structural, method, getter = onexport)]
    fn onexport(this: &JsAppProperties) -> js_sys::Function;
}

impl From<&JsAppProperties> for AppProperties {
    fn from(props: &JsAppProperties) -> Self {
        let onexport = props.onexport();
        Self {
            onexport: Callback::from(move |value: ExportedData| {
                let value =
                    JsValue::from_serde(&value).expect_throw("failed serializing `ExportedData`");
                onexport
                    .call1(&JsValue::null(), &value)
                    .expect_throw("failed calling `onexport` callback");
            }),
        }
    }
}

#[wasm_bindgen(js_name = runApp)]
pub fn run_app(props: &JsAppProperties) {
    let window = web_sys::window().expect_throw("no Window");
    let document = window.document().expect_throw("no Document");
    let element = document
        .query_selector("#app-root")
        .expect_throw("cannot get app root node")
        .expect_throw("cannot unwrap body node");

    yew::start_app_with_props_in_element::<App>(element, props.into());
}
