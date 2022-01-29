use regex::Regex;
use serde::Deserialize;

use std::{
    env,
    error::Error as StdError,
    fmt,
    fs::{self, File},
    io::Write as _,
    path::Path,
};

const MAIN_DEPENDENCIES: &[&str] = &["yew", "wasm-bindgen", "elastic-elgamal", "secret-tree"];

#[derive(Debug, Deserialize)]
struct RawPackage {
    name: String,
    version: String,
    source: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Lockfile {
    package: Vec<RawPackage>,
}

// **NB.** Needs to be synced with the `Package` struct in the crate.
#[derive(Debug)]
struct Package {
    name: String,
    version: String,
    rev: Option<String>,
    github_repo: Option<String>,
}

impl fmt::Display for Package {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Package {{ name: {name:?}, version: {version:?}, rev: {rev:?}, github_repo: {repo:?} }}",
            name = self.name,
            version = self.version,
            rev = self.rev,
            repo = self.github_repo
        )
    }
}

fn main() -> Result<(), Box<dyn StdError>> {
    let git_regex = Regex::new(r"^git.*github\.com/(?P<repo>.*)\?.*#(?P<rev>[0-9a-f]{40})$")
        .expect("cannot compile regex");

    let package_lock = fs::read_to_string("Cargo.lock")?;
    let lockfile: Lockfile = toml::from_str(&package_lock)?;
    let packages = lockfile.package.into_iter().filter_map(|package| {
        if MAIN_DEPENDENCIES.contains(&package.name.as_str()) {
            let (repo, rev) = match package
                .source
                .as_ref()
                .and_then(|source| git_regex.captures(source))
            {
                Some(captures) => (captures.name("repo"), captures.name("rev")),
                None => (None, None),
            };

            Some(Package {
                name: package.name,
                version: package.version,
                rev: rev.map(|m| m.as_str().to_owned()),
                github_repo: repo.map(|m| m.as_str().to_owned()),
            })
        } else {
            None
        }
    });
    let packages: Vec<_> = packages.collect();
    assert_eq!(
        packages.len(),
        MAIN_DEPENDENCIES.len(),
        "Main dependencies missing"
    );

    println!("cargo:rerun-if-changed=Cargo.lock");

    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir).join("main_deps.rs");
    let mut out_file = File::create(&out_path)?;
    writeln!(out_file, "&[")?;
    for package in packages {
        writeln!(out_file, "    {},", package)?;
    }
    writeln!(out_file, "]")?;

    Ok(())
}
