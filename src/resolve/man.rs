//! Man page one-liners via `apropos`, which reads the man database (never the tools).

use super::{Resolver, by_name};
use crate::model::{Fact, Meta, Source, Tool};
use std::collections::HashMap;
use std::process::Command;

pub struct Man(HashMap<String, Meta>);

impl Man {
    pub fn load() -> Self {
        let out = Command::new("apropos")
            .args(["-l", "."])
            .output()
            .map_or_default(|o| String::from_utf8_lossy(&o.stdout).into_owned());
        let mut best: HashMap<&str, (u8, &str)> = HashMap::new();
        out.lines().filter_map(parse).for_each(|(name, rank, desc)| {
            let better = best.get(name).is_none_or(|(r, _)| rank < *r);
            if better {
                best.insert(name, (rank, desc));
            }
        });
        Self(
            best.into_iter()
                .map(|(n, (_, d))| {
                    let description = Some(Fact::new(d, Source::Man));
                    (n.to_owned(), Meta { description, ..Meta::default() })
                })
                .collect(),
        )
    }
}

impl Resolver for Man {
    fn resolve(&self, tool: &Tool) -> Meta {
        by_name(&self.0, tool)
    }
}

/// `name (sect) - desc` into name, section rank (user commands first), desc.
fn parse(line: &str) -> Option<(&str, u8, &str)> {
    let (head, desc) = line.split_once(" - ")?;
    let (name, sect) = head.trim_end().split_once(" (")?;
    let rank = match sect.as_bytes().first()? {
        b'1' => 0,
        b'8' => 1,
        b'6' => 2,
        _ => 3,
    };
    Some((name, rank, desc.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_should_split_whatis_line() {
        assert_eq!(parse("ls (1)               - list directory contents"), Some(("ls", 0, "list directory contents")));
    }

    #[test]
    fn parse_should_rank_library_sections_last() {
        assert_eq!(parse("Socket (3perl) - constants").map(|p| p.1), Some(3));
    }
}
