//! Machine layer (`index.json` cache) and the merged registry view.

use crate::model::{Index, Tool};
use crate::{index, user};
use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::PathBuf;

/// `$var` or `$HOME/<fallback>`.
pub fn xdg(var: &str, fallback: &str) -> PathBuf {
    env::var_os(var)
        .filter(|v| !v.is_empty())
        .map_or_else(|| PathBuf::from(env::var_os("HOME").unwrap_or_default()).join(fallback), PathBuf::from)
}

fn index_path() -> PathBuf {
    xdg("XDG_DATA_HOME", ".local/share").join("treg/index.json")
}

/// Missing or unreadable cache is treated as empty.
fn load_index() -> Option<Index> {
    fs::read(index_path()).ok().and_then(|b| serde_json::from_slice(&b).ok())
}

fn save_index(index: &Index) -> Result<()> {
    let path = index_path();
    let tmp = path.with_extension("json.tmp");
    fs::create_dir_all(path.parent().context("index path has no parent")?)?;
    fs::write(&tmp, serde_json::to_vec(index)?).with_context(|| format!("writing {}", tmp.display()))?;
    fs::rename(&tmp, &path).with_context(|| format!("replacing {}", path.display()))
}

pub fn reindex() -> Result<Index> {
    let index = index::run(load_index().unwrap_or_default());
    save_index(&index)?;
    Ok(index)
}

pub fn prune() -> Result<Index> {
    let index = index::prune(load_index().unwrap_or_default());
    save_index(&index)?;
    Ok(index)
}

/// Merged registry: cached index (built on first run) overlaid with the user layer.
pub fn load() -> Result<Index> {
    let mut index = load_index().map_or_else(
        || {
            eprintln!("treg: first run, indexing $PATH");
            reindex()
        },
        Ok,
    )?;
    let mut user = user::load()?;
    index.tools.iter_mut().for_each(|t| {
        let found = t.names().find_map(|n| user.remove(n));
        t.user = found.unwrap_or_default();
    });
    index.tools.extend(user.into_iter().map(|(name, user)| Tool { name, user, ..Tool::default() }));
    index.tools.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(index)
}
