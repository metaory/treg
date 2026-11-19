//! Descriptions from a local tealdeer cache, read lazily per tool.

use super::Resolver;
use crate::model::{Fact, Meta, Source, Tool};
use crate::store::xdg;
use std::env;
use std::fs;
use std::path::PathBuf;

pub struct Tldr(Vec<PathBuf>);

impl Tldr {
    pub fn load() -> Self {
        let root = env::var_os("TEALDEER_CACHE_DIR")
            .map_or_else(|| xdg("XDG_CACHE_HOME", ".cache").join("tealdeer"), PathBuf::from)
            .join("tldr-pages/pages.en");
        Self(["common", "linux"].map(|p| root.join(p)).into_iter().filter(|d| d.is_dir()).collect())
    }
}

impl Resolver for Tldr {
    fn resolve(&self, tool: &Tool) -> Meta {
        let description = tool
            .names()
            .flat_map(|n| self.0.iter().map(move |d| d.join(format!("{n}.md"))))
            .find_map(|p| fs::read_to_string(p).ok())
            .and_then(|page| parse(&page).map(|d| Fact::new(d, Source::Tldr)));
        Meta { description, ..Meta::default() }
    }
}

fn parse(page: &str) -> Option<&str> {
    page.lines().find_map(|l| l.strip_prefix("> ")).map(str::trim)
}

#[cfg(test)]
mod tests {
    #[test]
    fn parse_should_take_first_quote_line() {
        assert_eq!(super::parse("# tar\n\n> Archiving utility.\n> More info."), Some("Archiving utility."));
    }
}
