use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Man,
    Tldr,
    Desktop,
    Pacman,
    User,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fact {
    pub value: String,
    pub source: Source,
}

impl Fact {
    pub fn new(value: impl Into<String>, source: Source) -> Self {
        Self { value: value.into(), source }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Meta {
    pub description: Option<Fact>,
    pub category: Option<Fact>,
    pub package: Option<String>,
}

impl Meta {
    /// Field-wise merge where `self` takes precedence.
    #[must_use]
    pub fn or(self, other: Self) -> Self {
        Self {
            description: self.description.or(other.description),
            category: self.category.or(other.category),
            package: self.package.or(other.package),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Exe {
    pub name: String,
    pub path: PathBuf,
    pub target: Option<PathBuf>,
    pub first_seen: u64,
    pub last_seen: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub exes: Vec<Exe>,
    pub auto: Meta,
    #[serde(skip)]
    pub user: Meta,
}

impl Tool {
    pub fn meta(&self) -> Meta {
        self.user.clone().or(self.auto.clone())
    }

    /// Primary name first, then every executable alias.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.name.as_str()).chain(self.exes.iter().map(|e| e.name.as_str()))
    }

    pub fn aliases(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.exes.iter().map(|e| e.name.as_str()).collect();
        names.sort_unstable();
        names.dedup();
        names.retain(|n| *n != self.name);
        names
    }

    pub fn is_manual(&self) -> bool {
        self.exes.is_empty()
    }

    pub fn is_stale(&self, indexed_at: u64) -> bool {
        !self.is_manual() && self.exes.iter().all(|e| e.last_seen < indexed_at)
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Index {
    pub indexed_at: u64,
    pub tools: Vec<Tool>,
}

pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_or_should_prefer_self_per_field() {
        let user = Meta { category: Some(Fact::new("Editor", Source::User)), ..Meta::default() };
        let auto = Meta {
            description: Some(Fact::new("vi improved", Source::Man)),
            category: Some(Fact::new("Utility", Source::Desktop)),
            package: Some("vim".into()),
        };
        let merged = user.or(auto);
        assert_eq!(
            (merged.description.map(|f| f.source), merged.category.map(|f| f.source)),
            (Some(Source::Man), Some(Source::User))
        );
    }
}
