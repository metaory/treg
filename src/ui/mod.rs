//! TEA loop: key -> `Msg` -> `App::update` -> `view::draw`.

mod app;
mod input;
mod status;
mod view;

use crate::store;
use crate::user::Field;
use anyhow::Result;
use app::App;
use input::Input;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

pub enum Mode {
    Browse,
    Add(Input),
    Edit { name: String, field: Field, input: Input },
}

pub enum Msg {
    Tick,
    Quit,
    Move(isize),
    Type(KeyEvent),
    Submit,
    Cancel,
    Edit(Field),
    Add,
    Reindex,
}

pub fn run() -> Result<()> {
    let mut app = App::new(store::load()?);
    ratatui::run(|terminal| event_loop(&mut app, terminal))
}

/// Errors from an update are shown in the status line instead of ending the session.
fn event_loop(app: &mut App, terminal: &mut DefaultTerminal) -> Result<()> {
    while !app.quit {
        terminal.draw(|f| view::draw(f, app))?;
        let ready = app.status.wait().map_or(Ok(true), event::poll)?;
        let msg = match ready.then(event::read).transpose()? {
            Some(Event::Key(k)) if k.kind == KeyEventKind::Press => msg(&app.mode, k),
            _ => Msg::Tick,
        };
        if let Err(e) = app.update(msg) {
            app.status.notify(format!("error: {e:#}"));
        }
    }
    Ok(())
}

fn msg(mode: &Mode, key: KeyEvent) -> Msg {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let browse = matches!(mode, Mode::Browse);
    match key.code {
        KeyCode::Char('c') if ctrl => Msg::Quit,
        KeyCode::Esc => Msg::Cancel,
        KeyCode::Enter => Msg::Submit,
        KeyCode::Up if browse => Msg::Move(-1),
        KeyCode::Down if browse => Msg::Move(1),
        KeyCode::Char('p') if ctrl && browse => Msg::Move(-1),
        KeyCode::Char('n') if ctrl && browse => Msg::Move(1),
        KeyCode::PageUp if browse => Msg::Move(-10),
        KeyCode::PageDown if browse => Msg::Move(10),
        KeyCode::Char('e') if ctrl && browse => Msg::Edit(Field::Category),
        KeyCode::Char('a') if ctrl && browse => Msg::Add,
        KeyCode::Char('r') if ctrl && browse => Msg::Reindex,
        _ => Msg::Type(key),
    }
}
