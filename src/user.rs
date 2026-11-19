//! User layer: hand-editable `tools.toml`, never written by the indexer.
//! A `[tool.<name>]` matching no executable is a manual tool.

use crate::model::{Fact, Meta, Source};
use crate::store::xdg;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use toml_edit::{DocumentMut, Item, Table, TableLike};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Description,
    Category,
}

impl Field {
    pub const fn key(self) -> &'static str {
        match self {
            Self::Description => "description",
            Self::Category => "category",
        }
    }
}

fn path() -> PathBuf {
    xdg("XDG_CONFIG_HOME", ".config").join("treg/tools.toml")
}

fn doc() -> Result<DocumentMut> {
    let text = fs::read_to_string(path()).unwrap_or_default();
    text.parse().with_context(|| format!("parsing {}", path().display()))
}

pub fn load() -> Result<HashMap<String, Meta>> {
    let doc = doc()?;
    let tools = doc.get("tool").and_then(Item::as_table_like);
    let fact = |t: &dyn TableLike, f: Field| t.get(f.key()).and_then(Item::as_str).map(|v| Fact::new(v, Source::User));
    Ok(tools
        .into_iter()
        .flat_map(|t| t.iter())
        .filter_map(|(name, item)| item.as_table_like().map(|t| (name, t)))
        .map(|(name, t)| {
            let meta =
                Meta { description: fact(t, Field::Description), category: fact(t, Field::Category), package: None };
            (name.to_owned(), meta)
        })
        .collect())
}

/// Sets a user field; an empty value removes it, and an emptied tool table goes too.
pub fn set(name: &str, field: Field, value: &str) -> Result<()> {
    let mut doc = doc()?;
    let tools = doc.entry("tool").or_insert_with(|| Item::Table(implicit()));
    let tool = tools[name].or_insert(Item::Table(Table::new()));
    let table = tool.as_table_like_mut().context("tool entry is not a table")?;
    table.remove(field.key());
    let value = value.trim();
    if !value.is_empty() {
        table.insert(field.key(), toml_edit::value(value));
    }
    if table.is_empty() {
        tools.as_table_like_mut().map(|t| t.remove(name));
    }
    let path = path();
    fs::create_dir_all(path.parent().context("config path has no parent")?)?;
    fs::write(&path, doc.to_string()).with_context(|| format!("writing {}", path.display()))
}

fn implicit() -> Table {
    let mut t = Table::new();
    t.set_implicit(true);
    t
}
