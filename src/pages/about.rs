//! About page.

use yew::{function_component, html, Html};

use super::PageMetadata;

#[derive(Debug)]
struct Package {
    name: &'static str,
    version: &'static str,
    rev: Option<&'static str>,
    github_repo: Option<&'static str>,
}

impl Package {
    fn view_dependency(&self) -> Html {
        let crate_link = format!(
            "https://crates.io/crates/{name}/{version}",
            name = self.name,
            version = self.version
        );
        html! {
            <li>
                { self.name }{ " version: " }
                <a href={crate_link} target="_blank">{ self.version }</a>
                {if let Some(rev) = self.rev {
                    let short_rev = &rev[..7];
                    html!{
                        <>
                            { " @ commit " }
                            {if let Some(repo) = self.github_repo {
                                let repo_link = format!(
                                    "https://github.com/{repo}/tree/{rev}",
                                    repo = repo,
                                    rev = rev
                                );
                                html! {
                                    <a href={repo_link} target="_blank">{ short_rev }</a>
                                }
                            } else {
                                html!{ short_rev }
                            }}
                        </>
                    }
                } else {
                    html!{}
                }}
            </li>
        }
    }
}

const MAIN_DEPENDENCIES: &[Package] = include!(concat!(env!("OUT_DIR"), "/main_deps.rs"));

fn view_dependencies() -> Html {
    MAIN_DEPENDENCIES
        .iter()
        .map(Package::view_dependency)
        .collect()
}

#[derive(Debug)]
struct GitInfo {
    commit_hash: &'static str,
}

impl GitInfo {
    const INSTANCE: Self = include!(concat!(env!("OUT_DIR"), "/git_info.rs"));

    fn view(&self) -> Html {
        let commit_link = format!(
            "https://github.com/slowli/elastic-elgamal-site/tree/{}",
            self.commit_hash
        );
        html! {
            <li>
                { "Deployed commit: " }
                <a href={commit_link} target="_blank">{ &self.commit_hash[..7] }</a>
            </li>
        }
    }
}

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

            <h3>{ "Build Info" }</h3>
            <p><em class="small">{ "Versions of key dependencies to simplify debugging." }</em></p>
            <ul>
                { GitInfo::INSTANCE.view() }
                { view_dependencies() }
            </ul>
        </>
    }
}
