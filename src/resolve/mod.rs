//! Passive metadata resolvers. Each bulk-loads its source once and degrades to
//! empty when the source is missing. Earlier resolvers win per field.

mod desktop;
mod man;
mod pacman;
mod tldr;

use crate::model::{Meta, Tool};
use std::collections::HashMap;

pub trait Resolver {
    fn resolve(&self, tool: &Tool) -> Meta;
}

pub fn all(tools: &[Tool]) -> Vec<Box<dyn Resolver>> {
    vec![
        Box::new(man::Man::load()),
        Box::new(tldr::Tldr::load()),
        Box::new(desktop::Desktop::load()),
        Box::new(pacman::Pacman::load(tools)),
    ]
}

pub fn meta(resolvers: &[Box<dyn Resolver>], tool: &Tool) -> Meta {
    resolvers.iter().map(|r| r.resolve(tool)).fold(Meta::default(), Meta::or)
}

/// First hit across the tool's primary name and aliases.
fn by_name(map: &HashMap<String, Meta>, tool: &Tool) -> Meta {
    tool.names().find_map(|n| map.get(n)).cloned().unwrap_or_default()
}
