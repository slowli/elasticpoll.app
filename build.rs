use regex::Regex;
use serde::Deserialize;

use std::{
    env,
    error::Error as StdError,
    fmt,
    fs::{self, File},
    io::Write as _,
    path::Path,
    process::Command,
    str,
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

// **NB.** Needs to be synced with the `GitInfo` struct in the crate.
#[derive(Debug)]
struct GitInfo {
    commit_hash: String,
}

impl fmt::Display for GitInfo {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "GitInfo {{ commit_hash: {commit_hash:?} }}",
            commit_hash = self.commit_hash,
        )
    }
}

fn record_git_info() -> Result<(), Box<dyn StdError>> {
    let output = Command::new("git")
        .args(["status", "--porcelain=v2", "--branch"])
        .output()?;

    let commit_hash_regex = Regex::new(r"\b(?P<hash>[\da-f]{40})$")?;
    let mut commit_hash = None;
    for line in output.stdout.split(|&ch| ch == b'\n') {
        if line.starts_with(b"# branch.oid") {
            let line = str::from_utf8(line)?;
            let captures = commit_hash_regex
                .captures(line)
                .ok_or("git commit hash not found for line")?;
            commit_hash = Some(captures["hash"].to_owned());
        }
    }

    let git_info = GitInfo {
        commit_hash: commit_hash.ok_or("commit hash not found")?,
    };
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir).join("git_info.rs");
    let mut out_file = File::create(&out_path)?;
    writeln!(out_file, "{}", git_info)?;

    println!("cargo:rerun-if-changed=.git/logs/HEAD");

    Ok(())
}

fn main() -> Result<(), Box<dyn StdError>> {
    let git_regex = Regex::new(r"^git.*github\.com/(?P<repo>.*)\?.*#(?P<rev>[\da-f]{40})$")?;

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

    record_git_info()
}
