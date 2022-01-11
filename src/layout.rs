//! Page layout.

use yew::{html, Html};
use yew_router::prelude::*;

use crate::components::Route;

pub fn header() -> Html {
    html! {
        <header class="body-header">
            <div class="container">
                <h1>
                    <Link<Route> to={ Route::Home } classes="d-block">{ "Voting" }</Link<Route>>
                </h1>
            </div>
        </header>
    }
}

pub fn footer() -> Html {
    html! {
        <footer class="page-footer small">
            <div class="row">
                <div class="col-md-9">
                    <p class="mb-2">
                        { "© 2022 Alex Ostrovski. Licensed under " }
                        <a rel="license" href="https://www.apache.org/licenses/LICENSE-2.0">
                            { "Apache 2.0" }
                        </a>
                    </p>
                    <p>
                        { "This site is open-source! " }
                        <a href="https://github.com/slowli/elastic-elgamal-site">
                            { "Contribute on GitHub" }
                        </a>
                    </p>
                </div>
                <div class="col-md-3">
                    <h5>{ "Useful links" }</h5>
                    <ul class="list-unstyled">
                        <li class="mb-1" title="About this website">
                            <Link<Route> to={ Route::About }>{ "About" }</Link<Route>>
                        </li>
                        <li>
                            <a href="https://crates.io/crates/elastic-elgamal"
                                title="Rust library powering this website"
                                target="_blank">
                                { "elastic-elgamal library" }
                            </a>
                        </li>
                    </ul>
                </div>
            </div>
        </footer>
    }
}
