mod discover;
mod index;
mod model;
mod resolve;
mod search;
mod store;
mod ui;
mod user;

use anyhow::Result;
use model::{Fact, Index};
use user::Field;

const USAGE: &str = "usage: treg [index | list | prune | add <name> <description> [category]]";

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        [] => ui::run(),
        ["index"] => store::reindex().map(|i| report("indexed", &i)),
        ["prune"] => store::prune().map(|i| report("pruned to", &i)),
        ["list"] => store::load().map(|i| list(&i)),
        ["add", name, desc, rest @ ..] => add(name, desc, rest.first().copied()),
        _ => {
            eprintln!("{USAGE}");
            Ok(())
        }
    }
}

fn report(verb: &str, index: &Index) {
    let stale = index.tools.iter().filter(|t| t.is_stale(index.indexed_at)).count();
    eprintln!("{verb} {} tools ({stale} stale)", index.tools.len());
}

fn list(index: &Index) {
    let value = |f: Option<Fact>| f.map_or_default(|f| f.value);
    index
        .tools
        .iter()
        .map(|t| (t, t.meta()))
        .map(|(t, m)| format!("{}\t{}\t{}", t.name, value(m.description), value(m.category)))
        .for_each(|line| println!("{line}"));
}

fn add(name: &str, desc: &str, category: Option<&str>) -> Result<()> {
    user::set(name, Field::Description, desc)?;
    category.map_or(Ok(()), |c| user::set(name, Field::Category, c))
}
