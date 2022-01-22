//! About page.

use yew::{function_component, html};

use super::PageMetadata;

// TODO: add build details
#[function_component(About)]
pub fn about_page() -> Html {
    let metadata = PageMetadata {
        title: "About the app".to_owned(),
        description: "??? is a fully contained WASM web app allowing to hold polls \
            in a cryptographically secure and private manner. \
            This page lists main technologies about the app and some debugging info."
            .to_owned(),
        is_root: false,
    };

    html! {
        <>
            { metadata.view() }
            <p class="lead mb-4">
                { "This web app was made possible with the help of following awesome tech:" }
            </p>
            <ul>
                <li>
                    <a href="https://developer.mozilla.org/en-US/docs/WebAssembly/Concepts">
                        { "WASM" }
                    </a>
                    { " – the virtual machine for the Web" }
                </li>
                <li>
                    <a href="https://www.rust-lang.org/">{ "Rust programming language" }</a>
                    { " and " }
                    <a href="https://rustwasm.github.io/">
                        { "Rust → WASM toolchain" }
                    </a>
                    { " allowing to bring Rust safety and performance to the browser" }
                </li>
                <li>
                    <a href="https://crates.io/crates/elastic-elgamal">
                        { "elastic-elgamal" }
                    </a>
                    { " Rust library for cryptographically secure polling" }
                </li>
                <li>
                    <a href="https://yew.rs/">{ "Yew framework" }</a>
                    { " bringing Rust to the front-end" }
                </li>
            </ul>
            <p>
                { "This website is fully open source! See " }
                <a href="https://github.com/slowli/elastic-elgamal-site">{ "its source code" }</a>
                { " for the full list of dependencies and feel welcome to submit changes or \
                  suggest new functionality." }
            </p>
        </>
    }
}
