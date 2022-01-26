use wasm_bindgen_test::wasm_bindgen_test_configure;

#[cfg(feature = "testing")]
mod pages;
mod poll;

wasm_bindgen_test_configure!(run_in_browser);
