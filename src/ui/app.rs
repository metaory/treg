use super::input::Input;
use super::status::Status;
use super::{Mode, Msg};
use crate::model::{Index, Tool};
use crate::search::Search;
use crate::store;
use crate::user::{self, Field};
use anyhow::Result;
use ratatui::widgets::ListState;

pub struct App {
    pub index: Index,
    pub query: Input,
    pub hits: Vec<usize>,
    pub list: ListState,
    pub mode: Mode,
    pub status: Status,
    pub quit: bool,
    search: Search,
}

impl App {
    pub fn new(index: Index) -> Self {
        Self {
            search: Search::new(&index.tools),
            hits: (0..index.tools.len()).collect(),
            list: ListState::default().with_selected(Some(0)),
            index,
            query: Input::default(),
            mode: Mode::Browse,
            status: Status::default(),
            quit: false,
        }
    }

    pub fn selected(&self) -> Option<&Tool> {
        self.list.selected().and_then(|i| self.hits.get(i)).map(|&i| &self.index.tools[i])
    }

    pub fn update(&mut self, msg: Msg) -> Result<()> {
        match (msg, &mut self.mode) {
            (Msg::Tick, _) => return self.tick(),
            (Msg::Move(d), _) => {
                let last = self.hits.len().saturating_sub(1);
                self.list.select(self.list.selected().map(|i| i.saturating_add_signed(d).min(last)));
            }
            (Msg::Type(k), Mode::Browse) if self.query.handle(k) => self.refilter(),
            (Msg::Type(k), Mode::Add(input) | Mode::Edit { input, .. }) => {
                input.handle(k);
            }
            (Msg::Cancel, Mode::Browse) if !self.query.text.is_empty() => {
                self.query = Input::default();
                self.refilter();
            }
            (Msg::Quit, _) | (Msg::Cancel, Mode::Browse) => self.quit = true,
            (Msg::Cancel, _) => self.mode = Mode::Browse,
            (Msg::Edit(field), Mode::Browse) => self.edit(field),
            (Msg::Add, Mode::Browse) => self.mode = Mode::Add(Input::default()),
            (Msg::Submit, Mode::Browse) => self.edit(Field::Description),
            (Msg::Submit, Mode::Add(input)) if !input.text.trim().is_empty() => {
                let name = input.text.trim().to_owned();
                self.mode = Mode::Edit { name, field: Field::Description, input: Input::default() };
            }
            (Msg::Submit, Mode::Edit { name, field, input }) => {
                user::set(name, *field, &input.text)?;
                let (name, saved) = (std::mem::take(name), field.key());
                self.mode = Mode::Browse;
                self.status.notify(format!("saved {saved} of {name}"));
                self.reload(&name)?;
            }
            (Msg::Reindex, _) => self.status.reindex(),
            _ => {}
        }
        Ok(())
    }

    /// Reloads once a background reindex finishes.
    fn tick(&mut self) -> Result<()> {
        let Some(done) = self.status.tick() else { return Ok(()) };
        self.status.notify(format!("indexed {} tools", done?.tools.len()));
        let name = self.selected().map_or_default(|t| t.name.clone());
        self.reload(&name)
    }

    fn edit(&mut self, field: Field) {
        let Some(tool) = self.selected() else { return };
        let meta = tool.meta();
        let current = match field {
            Field::Description => meta.description,
            Field::Category => meta.category,
        };
        let input = Input::new(&current.map_or_default(|f| f.value));
        self.mode = Mode::Edit { name: tool.name.clone(), field, input };
    }

    fn refilter(&mut self) {
        self.hits = self.search.run(&self.query.text);
        self.list.select(Some(0));
    }

    /// Reloads both layers and keeps the cursor on `name` when it is still listed.
    fn reload(&mut self, name: &str) -> Result<()> {
        self.index = store::load()?;
        self.search = Search::new(&self.index.tools);
        self.hits = self.search.run(&self.query.text);
        let at = self.hits.iter().position(|&i| self.index.tools[i].name == name);
        self.list.select(at.or(Some(0)));
        Ok(())
    }
}
