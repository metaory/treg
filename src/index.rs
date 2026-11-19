//! Incremental sync: discover, resolve outside any write, merge with the previous index.

use crate::discover::discover;
use crate::model::{Index, Tool, now};
use crate::resolve;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

pub fn run(prev: Index) -> Index {
    let now = now();
    let found = discover(now);
    let resolvers = resolve::all(&found);
    let tools = found.into_iter().map(|t| Tool { auto: resolve::meta(&resolvers, &t), ..t }).collect();
    Index { indexed_at: now, tools: merge(prev.tools, tools) }
}

/// Carries `first_seen` forward and keeps vanished executables as stale.
fn merge(prev: Vec<Tool>, mut tools: Vec<Tool>) -> Vec<Tool> {
    let first_seen: HashMap<PathBuf, u64> =
        prev.iter().flat_map(|t| &t.exes).map(|e| (e.path.clone(), e.first_seen)).collect();
    tools.iter_mut().flat_map(|t| &mut t.exes).for_each(|e| {
        e.first_seen = first_seen.get(&e.path).copied().unwrap_or(e.first_seen);
    });
    let seen: HashSet<PathBuf> = tools.iter().flat_map(|t| &t.exes).map(|e| e.path.clone()).collect();
    let by_name: HashMap<String, usize> = tools.iter().enumerate().map(|(i, t)| (t.name.clone(), i)).collect();
    let stale = prev.into_iter().filter_map(|mut t| {
        t.exes.retain(|e| !seen.contains(&e.path));
        (!t.exes.is_empty()).then_some(t)
    });
    let orphans: Vec<Tool> = stale
        .filter_map(|t| match by_name.get(&t.name) {
            Some(&i) => {
                tools[i].exes.extend(t.exes);
                None
            }
            None => Some(t),
        })
        .collect();
    tools.extend(orphans);
    tools
}

/// Drops stale executables and tools left without any.
pub fn prune(mut index: Index) -> Index {
    index.tools.iter_mut().for_each(|t| t.exes.retain(|e| e.last_seen >= index.indexed_at));
    index.tools.retain(|t| !t.exes.is_empty());
    index
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Exe;

    fn tool(name: &str, path: &str, seen: u64) -> Tool {
        let exe = Exe { name: name.into(), path: path.into(), target: None, first_seen: seen, last_seen: seen };
        Tool { name: name.into(), exes: vec![exe], ..Tool::default() }
    }

    #[test]
    fn merge_should_keep_first_seen() {
        let merged = merge(vec![tool("a", "/a", 1)], vec![tool("a", "/a", 5)]);
        assert_eq!(merged[0].exes[0].first_seen, 1);
    }

    #[test]
    fn merge_should_keep_vanished_as_stale() {
        let merged = merge(vec![tool("gone", "/gone", 1)], vec![tool("a", "/a", 5)]);
        assert!(merged[1].is_stale(5));
    }
}
