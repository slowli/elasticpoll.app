//! Misc utils.

use serde::{
    de::{DeserializeOwned, Error as _, SeqAccess, Visitor},
    Deserializer, Serialize, Serializer,
};
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::{Event, HtmlInputElement, HtmlTextAreaElement};

use std::{fmt, marker::PhantomData};

use crate::poll::PublicKey;

pub(crate) struct VecHelper<T, const MIN: usize, const MAX: usize>(PhantomData<T>);

impl<T, const MIN: usize, const MAX: usize> VecHelper<T, MIN, MAX>
where
    T: Serialize + DeserializeOwned,
{
    fn new() -> Self {
        Self(PhantomData)
    }

    pub fn serialize<S>(values: &[T], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        debug_assert!(values.len() >= MIN && values.len() <= MAX);
        serializer.collect_seq(values.iter())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(Self::new())
    }
}

impl<'de, T, const MIN: usize, const MAX: usize> Visitor<'de> for VecHelper<T, MIN, MAX>
where
    T: DeserializeOwned,
{
    type Value = Vec<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "at least {} and at most {} items", MIN, MAX)
    }

    fn visit_seq<S>(self, mut access: S) -> Result<Self::Value, S::Error>
    where
        S: SeqAccess<'de>,
    {
        let mut scalars: Vec<T> = if let Some(size) = access.size_hint() {
            if size < MIN || size > MAX {
                return Err(S::Error::invalid_length(size, &self));
            }
            Vec::with_capacity(size)
        } else {
            Vec::new()
        };

        while let Some(value) = access.next_element::<T>()? {
            scalars.push(value);
        }
        if scalars.len() >= MIN && scalars.len() <= MAX {
            Ok(scalars)
        } else {
            Err(S::Error::invalid_length(scalars.len(), &self))
        }
    }
}

/// Returns `window.localStorage` object.
pub(crate) fn local_storage() -> web_sys::Storage {
    web_sys::window()
        .expect_throw("no window")
        .local_storage()
        .expect_throw("failed to get LocalStorage")
        .expect_throw("no LocalStorage")
}

pub(crate) fn value_from_event(event: &Event) -> String {
    get_event_target::<HtmlTextAreaElement>(event).value()
}

pub(crate) fn value_from_input_event(event: &Event) -> String {
    get_event_target::<HtmlInputElement>(event).value()
}

pub(crate) fn get_event_target<E: JsCast>(event: &Event) -> E {
    let target = event.target().expect_throw("no target for event");
    target
        .dyn_into::<E>()
        .expect_throw("unexpected target for event")
}

pub(crate) trait Encode {
    fn encode(&self) -> String;
}

impl Encode for PublicKey {
    fn encode(&self) -> String {
        base64::encode_config(self.as_bytes(), base64::URL_SAFE_NO_PAD)
    }
}
