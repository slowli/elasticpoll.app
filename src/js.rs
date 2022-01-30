//! Types involved in interaction with the JS host.

use js_sys::Promise;
use serde::{Deserialize, Serialize};
use wasm_bindgen::{prelude::*, UnwrapThrowExt};
use yew::Callback;

use std::{fmt, rc::Rc};

use crate::{pages::AppProperties, poll::SecretManager};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedData {
    #[serde(rename = "type")]
    pub ty: ExportedDataType,
    pub data: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportedDataType {
    PollSpec,
    PollState,
    Application,
    Vote,
    TallierShare,
}

/// Encapsulates host-side password-based encryption operations.
pub trait PasswordBasedCrypto {
    /// Seals `secret_bytes` with `password` encryption.
    ///
    /// The promise must return a string (a password-encrypted box).
    fn seal(&self, password: &str, secret_bytes: &[u8]) -> Promise;

    /// Returns the cached value of the secret, or `null` if it is not cached yet.
    ///
    /// The promise must return a [`Uint8Array`] or `null`.
    fn cached(&self) -> Promise;

    /// Opens a previously sealed box with the specified `password`.
    ///
    /// The promise must return a [`Uint8Array`], or throw an error if decryption
    /// is not successful.
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

    #[wasm_bindgen(structural, method, js_name = getCachedBox)]
    fn cached_box(this: &JsAppProperties) -> Promise;

    #[wasm_bindgen(structural, method, js_name = openBox)]
    fn open_box(this: &JsAppProperties, password: &str, encrypted: &str) -> Promise;

    #[wasm_bindgen(structural, method, js_name = sealBox)]
    fn seal_box(this: &JsAppProperties, password: &str, secret_bytes: &[u8]) -> Promise;
}

impl PasswordBasedCrypto for JsAppProperties {
    fn seal(&self, password: &str, secret_bytes: &[u8]) -> Promise {
        self.seal_box(password, secret_bytes)
    }

    fn cached(&self) -> Promise {
        self.cached_box()
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
