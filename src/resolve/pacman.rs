//! Owning package from the plain-text pacman local db, read directly (no subprocess).

use super::Resolver;
use crate::model::{Fact, Meta, Source, Tool};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

const DB: &str = "/var/lib/pacman/local";

pub struct Pacman(HashMap<PathBuf, Meta>);

impl Pacman {
    pub fn load(tools: &[Tool]) -> Self {
        let wanted: HashSet<&Path> = tools
            .iter()
            .flat_map(|t| &t.exes)
            .flat_map(|e| std::iter::once(e.path.as_path()).chain(e.target.as_deref()))
            .collect();
        let pkgs = fs::read_dir(DB).map(|rd| rd.filter_map(Result::ok).map(|e| e.path()).collect());
        Self(pkgs.unwrap_or_else(|_| vec![]).iter().flat_map(|dir| package(dir, &wanted)).collect())
    }
}

impl Resolver for Pacman {
    fn resolve(&self, tool: &Tool) -> Meta {
        tool.exes
            .iter()
            .flat_map(|e| e.target.iter().chain([&e.path]))
            .find_map(|p| self.0.get(p))
            .cloned()
            .unwrap_or_default()
    }
}

/// `(path, meta)` for every wanted executable owned by the package in `dir`.
fn package(dir: &Path, wanted: &HashSet<&Path>) -> Vec<(PathBuf, Meta)> {
    let read = |f: &str| fs::read_to_string(dir.join(f)).unwrap_or_default();
    let desc = read("desc");
    let fields = blocks(&desc);
    let first = |k: &str| fields.get(k).and_then(|v| v.first()).copied();
    let meta = Meta {
        description: first("%DESC%").map(|d| Fact::new(d, Source::Pacman)),
        category: first("%GROUPS%").map(|g| Fact::new(g, Source::Pacman)),
        package: first("%NAME%").map(str::to_owned),
    };
    let files = read("files");
    blocks(&files)
        .get("%FILES%")
        .into_iter()
        .flatten()
        .map(|f| Path::new("/").join(f))
        .filter(|p| wanted.contains(p.as_path()))
        .map(|p| (p, meta.clone()))
        .collect()
}

/// Blank-line separated `%KEY%` blocks into key and value lines.
fn blocks(text: &str) -> HashMap<&str, Vec<&str>> {
    text.split("\n\n")
        .filter_map(|b| {
            let mut lines = b.lines();
            lines.next().map(|k| (k.trim(), lines.collect()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn blocks_should_split_sections() {
        let b = super::blocks("%NAME%\ncoreutils\n\n%GROUPS%\nbase\nxorg\n");
        assert_eq!((b["%NAME%"].clone(), b["%GROUPS%"].len()), (vec!["coreutils"], 2));
    }
}
