//! Freedesktop `.desktop` entries: description from Comment, category from Categories.

use super::{Resolver, by_name};
use crate::model::{Fact, Meta, Source, Tool};
use crate::store::xdg;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const MAIN: [&str; 13] = [
    "AudioVideo",
    "Audio",
    "Video",
    "Development",
    "Education",
    "Game",
    "Graphics",
    "Network",
    "Office",
    "Science",
    "Settings",
    "System",
    "Utility",
];

pub struct Desktop(HashMap<String, Meta>);

impl Desktop {
    pub fn load() -> Self {
        let data = env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
        let dirs = std::iter::once(xdg("XDG_DATA_HOME", ".local/share"))
            .chain(env::split_paths(&data))
            .map(|d| d.join("applications"));
        let mut map = HashMap::new();
        dirs.flat_map(|d| entries(&d)).filter_map(|p| fs::read_to_string(p).ok()).filter_map(|s| parse(&s)).for_each(
            |(name, meta)| {
                map.entry(name).or_insert(meta);
            },
        );
        Self(map)
    }
}

impl Resolver for Desktop {
    fn resolve(&self, tool: &Tool) -> Meta {
        by_name(&self.0, tool)
    }
}

fn entries(dir: &Path) -> Vec<PathBuf> {
    let is_entry = |p: &PathBuf| p.extension().is_some_and(|e| e == "desktop");
    fs::read_dir(dir).map_or_default(|rd| rd.filter_map(Result::ok).map(|e| e.path()).filter(is_entry).collect())
}

/// Keys of the `[Desktop Entry]` group only; later groups are actions.
fn parse(text: &str) -> Option<(String, Meta)> {
    let keys: HashMap<&str, &str> = text
        .lines()
        .skip_while(|l| l.trim() != "[Desktop Entry]")
        .skip(1)
        .take_while(|l| !l.starts_with('['))
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.trim(), v.trim()))
        .collect();
    let name = exec_name(keys.get("Exec")?)?;
    let fact = |v: &str| Fact::new(v, Source::Desktop);
    let description = keys.get("Comment").or_else(|| keys.get("GenericName")).map(|v| fact(v));
    let category = keys.get("Categories").and_then(|c| c.split(';').find(|c| MAIN.contains(c))).map(fact);
    Some((name, Meta { description, category, package: None }))
}

/// Basename of the first Exec token, skipping `env` and `VAR=value` prefixes.
fn exec_name(exec: &str) -> Option<String> {
    let cmd = exec.split_whitespace().find(|t| *t != "env" && !t.contains('='))?.trim_matches('"');
    Path::new(cmd).file_name()?.to_str().map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exec_name_should_skip_env_prefix() {
        assert_eq!(exec_name("env GDK_BACKEND=x11 /usr/bin/audacity %F").as_deref(), Some("audacity"));
    }

    #[test]
    fn parse_should_pick_main_category() {
        let (_, meta) =
            parse("[Desktop Entry]\nExec=arandr\nComment=Screen layout\nCategories=Settings;HardwareSettings;")
                .unwrap();
        assert_eq!(meta.category.map(|c| c.value).as_deref(), Some("Settings"));
    }
}
