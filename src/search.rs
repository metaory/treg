//! Fuzzy ranking (fzf syntax) over name, aliases, description, category and package.

use crate::model::Tool;
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

pub struct Search {
    matcher: Matcher,
    hay: Vec<(String, String)>,
}

impl Search {
    pub fn new(tools: &[Tool]) -> Self {
        let hay = tools.iter().map(|t| (t.name.clone(), haystack(t))).collect();
        Self { matcher: Matcher::new(Config::DEFAULT), hay }
    }

    /// Indices into the tools, best first; every tool in order for an empty query.
    pub fn run(&mut self, query: &str) -> Vec<usize> {
        let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);
        let mut buf = Vec::new();
        let mut score = |s: &str| pattern.score(Utf32Str::new(s, &mut buf), &mut self.matcher);
        let mut hits: Vec<(usize, u32)> = self
            .hay
            .iter()
            .enumerate()
            .filter_map(|(i, (name, hay))| {
                let by_name = score(name).map(|s| s * 2);
                by_name.max(score(hay)).map(|s| (i, s))
            })
            .collect();
        hits.sort_by_key(|&(i, s)| (std::cmp::Reverse(s), i));
        hits.into_iter().map(|(i, _)| i).collect()
    }
}

fn haystack(tool: &Tool) -> String {
    let meta = tool.meta();
    [
        Some(tool.name.clone()),
        Some(tool.aliases().join(" ")),
        meta.description.map(|f| f.value),
        meta.category.map(|f| f.value),
        meta.package,
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Fact, Meta, Source};

    fn tool(name: &str, desc: &str) -> Tool {
        let description = Some(Fact::new(desc, Source::User));
        Tool { name: name.into(), user: Meta { description, ..Meta::default() }, ..Tool::default() }
    }

    #[test]
    fn run_should_find_by_description() {
        let tools = [tool("qbt", "torrent downloader"), tool("vim", "editor")];
        assert_eq!(Search::new(&tools).run("torrent"), vec![0]);
    }

    #[test]
    fn run_should_rank_name_over_description() {
        let tools = [tool("x", "uses vim keys"), tool("vim", "editor")];
        assert_eq!(Search::new(&tools).run("vim")[0], 1);
    }
}
