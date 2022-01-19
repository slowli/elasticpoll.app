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

use js_sys::Promise;
use wasm_bindgen::{prelude::*, UnwrapThrowExt};
use yew::Callback;

use std::{fmt, rc::Rc};

mod components;
mod layout;
mod poll;
mod rng;
mod utils;

use self::{
    components::{App, AppProperties, ExportedData},
    poll::SecretManager,
};

/// Encapsulates host-side password-based encryption operations.
pub trait PasswordBasedCrypto {
    /// The promise must return a string.
    fn seal(&self, password: &str, secret_bytes: &[u8]) -> Promise;
    /// The promise must return a [`Uint8Array`].
    fn open(&self, password: &str, encrypted: &str) -> Promise;
}

impl fmt::Debug for dyn PasswordBasedCrypto {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("PasswordBasedCrypto").finish()
    }
}

pub trait ManageModals {
    fn hide_modal(&self, element_id: &str);
}

impl fmt::Debug for dyn ManageModals {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("ManageModals").finish()
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = AppProperties)]
    pub type JsAppProperties;

    #[wasm_bindgen(structural, method, js_name = hideModal)]
    fn hide_modal(this: &JsAppProperties, element_id: &str);

    #[wasm_bindgen(structural, method)]
    fn onexport(this: &JsAppProperties, data: JsValue);

    #[wasm_bindgen(structural, method, js_name = openBox)]
    fn open_box(this: &JsAppProperties, password: &str, encrypted: &str) -> Promise;

    #[wasm_bindgen(structural, method, js_name = sealBox)]
    fn seal_box(this: &JsAppProperties, password: &str, secret_bytes: &[u8]) -> Promise;
}

impl PasswordBasedCrypto for JsAppProperties {
    fn seal(&self, password: &str, secret_bytes: &[u8]) -> Promise {
        self.seal_box(password, secret_bytes)
    }

    fn open(&self, password: &str, encrypted: &str) -> Promise {
        self.open_box(password, encrypted)
    }
}

impl ManageModals for JsAppProperties {
    fn hide_modal(&self, element_id: &str) {
        self.hide_modal(element_id);
    }
}

impl From<JsAppProperties> for AppProperties {
    fn from(props: JsAppProperties) -> Self {
        let props = Rc::new(props);
        let onexport_props = Rc::clone(&props);

        Self {
            onexport: Callback::from(move |data: ExportedData| {
                let data =
                    JsValue::from_serde(&data).expect_throw("cannot serialize `ExportedData`");
                onexport_props.onexport(data);
            }),
            modals: Rc::clone(&props) as Rc<dyn ManageModals>,
            secrets: Rc::new(SecretManager::new(props)),
        }
    }
}

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
