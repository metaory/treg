//! Builds the executable inventory from `$PATH`.
//! Only stat, readdir and canonicalize are used: nothing found here is ever executed.

use crate::model::{Exe, Tool};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

struct Found {
    name: String,
    path: PathBuf,
    canon: PathBuf,
}

pub fn discover(now: u64) -> Vec<Tool> {
    group(path_dirs().iter().flat_map(|d| scan(d)).collect(), now)
}

/// Canonical, de-duplicated `$PATH` dirs in lookup order (`/bin` and `/usr/bin` collapse).
fn path_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> =
        env::var_os("PATH").map_or_default(|p| env::split_paths(&p).filter_map(|d| fs::canonicalize(d).ok()).collect());
    let mut seen = std::collections::HashSet::new();
    dirs.retain(|d| seen.insert(d.clone()));
    dirs
}

fn scan(dir: &Path) -> Vec<Found> {
    let Ok(entries) = fs::read_dir(dir) else {
        return vec![];
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|e| {
            let path = e.path();
            let raw = e.file_name().to_str()?.to_owned();
            let meta = fs::metadata(&path).ok()?;
            marked(&meta, &raw).then(|| fs::canonicalize(&path).ok()).flatten().map(|canon| Found {
                name: command(&raw),
                path,
                canon,
            })
        })
        .collect()
}

/// Groups entries sharing a canonical target (symlinks) or a name (shadowed copies).
fn group(found: Vec<Found>, now: u64) -> Vec<Tool> {
    let mut tools: Vec<Vec<Found>> = vec![];
    let mut by_canon: HashMap<PathBuf, usize> = HashMap::new();
    let mut by_name: HashMap<String, usize> = HashMap::new();
    for f in found {
        let idx = by_canon.get(&f.canon).or_else(|| by_name.get(&f.name)).copied().unwrap_or_else(|| {
            tools.push(vec![]);
            tools.len() - 1
        });
        by_canon.insert(f.canon.clone(), idx);
        by_name.insert(f.name.clone(), idx);
        tools[idx].push(f);
    }
    tools.into_iter().map(|g| tool(g, now)).collect()
}

fn tool(group: Vec<Found>, now: u64) -> Tool {
    let is_target = |f: &&Found| f.canon.file_name().and_then(|n| n.to_str()).is_some_and(|n| command(n) == f.name);
    let name = group.iter().find(is_target).unwrap_or(&group[0]).name.clone();
    let exes = group
        .into_iter()
        .map(|f| Exe {
            target: (f.canon != f.path).then_some(f.canon),
            name: f.name,
            path: f.path,
            first_seen: now,
            last_seen: now,
        })
        .collect();
    Tool { name, exes, ..Tool::default() }
}

/// Prompt name. Windows keeps a `PATHEXT` suffix on disk (`git.exe`).
fn command(name: &str) -> String {
    #[cfg(windows)]
    let name = Path::new(name).file_stem().and_then(|s| s.to_str()).unwrap_or(name);
    name.to_owned()
}

#[cfg(unix)]
fn marked(meta: &fs::Metadata, _: &str) -> bool {
    use std::os::unix::fs::PermissionsExt;
    meta.is_file() && meta.permissions().mode() & 0o111 != 0
}

#[cfg(windows)]
fn marked(meta: &fs::Metadata, name: &str) -> bool {
    let ext = Path::new(name).extension().and_then(|e| e.to_str());
    meta.is_file()
        && ext.is_some_and(|ext| pathext().split(';').any(|p| p.trim_start_matches('.').eq_ignore_ascii_case(ext)))
}

#[cfg(windows)]
fn pathext() -> String {
    env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into())
}

#[cfg(not(any(unix, windows)))]
compile_error!("executable discovery supports unix and windows");

#[cfg(test)]
mod tests {
    use super::*;

    fn found(name: &str, path: &str, canon: &str) -> Found {
        Found { name: name.into(), path: path.into(), canon: canon.into() }
    }

    #[test]
    fn group_should_merge_symlinks_under_target_name() {
        let tools = group(vec![found("vi", "/b/vi", "/b/vim"), found("vim", "/b/vim", "/b/vim")], 0);
        assert_eq!((tools.len(), tools[0].name.as_str(), tools[0].aliases()), (1, "vim", vec!["vi"]));
    }

    #[test]
    fn group_should_merge_shadowed_names() {
        let tools = group(vec![found("rg", "/a/rg", "/a/rg"), found("rg", "/b/rg", "/b/rg")], 0);
        assert_eq!((tools.len(), tools[0].exes.len()), (1, 2));
    }
}
