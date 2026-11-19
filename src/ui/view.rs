use super::Mode;
use super::app::App;
use crate::model::{Fact, Tool};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, List, ListItem, Paragraph, Wrap};

const KEYS: [(&str, &str); 6] = [
    ("enter", "describe"),
    ("^e", "category"),
    ("^a", "add"),
    ("^r", "reindex"),
    ("↑↓", "move"),
    ("esc", "clear/quit"),
];
pub fn draw(frame: &mut Frame, app: &mut App) {
    let [top, main, bottom] =
        Layout::vertical([Constraint::Length(1), Constraint::Fill(1), Constraint::Length(1)]).areas(frame.area());
    let [left, right] = Layout::horizontal([Constraint::Percentage(45), Constraint::Fill(1)]).areas(main);
    prompt(frame, top, "› ".bold().magenta(), &app.query.text, app.query.cursor);
    results(frame, left, app);
    let details = app.selected().map_or_default(|t| details(t, app.index.indexed_at));
    let block = Block::bordered().border_type(BorderType::Thick).title(" details ".bold());
    frame.render_widget(Paragraph::new(details).wrap(Wrap { trim: false }).block(block), right);
    match &app.mode {
        Mode::Browse => frame.render_widget(status(app), bottom),
        Mode::Add(i) => prompt(frame, bottom, "new tool name › ".bold().cyan(), &i.text, i.cursor),
        Mode::Edit { name, field, input } => {
            let label = format!("{name} {} › ", field.key()).bold().cyan();
            prompt(frame, bottom, label, &input.text, input.cursor);
        }
    }
}

fn prompt(frame: &mut Frame, area: Rect, label: Span, text: &str, cursor: usize) {
    let x = area.x + u16::try_from(label.width() + cursor).unwrap_or(u16::MAX);
    frame.render_widget(Line::from(vec![label, text.into()]), area);
    frame.set_cursor_position(Position::new(x.min(area.right().saturating_sub(1)), area.y));
}

fn results(frame: &mut Frame, area: Rect, app: &mut App) {
    let stale_at = app.index.indexed_at;
    let items: Vec<ListItem> = app
        .hits
        .iter()
        .map(|&i| &app.index.tools[i])
        .map(|t| {
            let name = Span::from(t.name.clone()).bold();
            let name = match (t.is_manual(), t.is_stale(stale_at)) {
                (true, _) => name.magenta(),
                (_, true) => name.dark_gray().crossed_out(),
                _ => name.cyan(),
            };
            let desc = t.meta().description.map_or_default(|f| f.value);
            ListItem::new(Line::from(vec![name, "  ".into(), desc.dim()]))
        })
        .collect();
    let title = format!(" {}/{} ", app.hits.len(), app.index.tools.len());
    let block = Block::bordered().border_type(BorderType::Thick).title(title.bold());
    let list = List::new(items).block(block).highlight_style(Style::new().reversed());
    frame.render_stateful_widget(list, area, &mut app.list);
}

fn details(tool: &Tool, indexed_at: u64) -> Vec<Line<'static>> {
    let meta = tool.meta();
    let fact = |label: &'static str, f: Option<Fact>| {
        let (value, source) = f.map_or((String::new(), String::new()), |f| (f.value, format!("  {:?}", f.source)));
        Line::from(vec![format!("{label:<12}").dim(), value.into(), source.to_lowercase().dark_gray()])
    };
    let plain = |label: &'static str, value: String| Line::from(vec![format!("{label:<12}").dim(), value.into()]);
    let state = match (tool.is_manual(), tool.is_stale(indexed_at)) {
        (true, _) => "manual".magenta(),
        (_, true) => "stale".red(),
        _ => "installed".green(),
    };
    let paths = tool.exes.iter().map(|e| {
        let target = e.target.as_ref().map_or_default(|t| format!(" → {}", t.display()));
        let gone = if e.last_seen < indexed_at { Style::new().red() } else { Style::new() };
        Line::from(vec![Span::styled(e.path.display().to_string(), gone), target.dim()])
    });
    [
        Line::from(vec![tool.name.clone().bold().cyan(), "  ".into(), state]),
        Line::default(),
        fact("description", meta.description),
        fact("category", meta.category),
        plain("package", meta.package.unwrap_or_default()),
        plain("aliases", tool.aliases().join(" ")),
        Line::default(),
    ]
    .into_iter()
    .chain(paths)
    .collect()
}

/// Spinner while indexing, else the timed status, else key help.
fn status(app: &App) -> Line<'_> {
    let spin =
        app.status.spinner().map(|c| Line::from(vec![format!(" {c} ").bold().magenta(), "indexing $PATH".dim()]));
    let text = app.status.message().map(|text| Line::from(format!(" {text}").yellow()));
    spin.or(text).unwrap_or_else(|| {
        KEYS.iter().flat_map(|(k, v)| [format!(" {k} ").bold().cyan(), format!("{v} ").dim()]).collect()
    })
}
