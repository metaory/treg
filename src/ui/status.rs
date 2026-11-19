//! Time-driven status bar state: a self-expiring message and a background reindex.

use crate::model::Index;
use crate::store;
use anyhow::{Result, anyhow};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

const TTL: Duration = Duration::from_secs(3);
const SPIN: Duration = Duration::from_millis(80);
const SPINNER: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

#[derive(Default)]
pub struct Status {
    message: Option<(String, Instant)>,
    job: Option<(Receiver<Result<Index>>, Instant)>,
}

impl Status {
    pub fn notify(&mut self, text: String) {
        self.message = Some((text, Instant::now()));
    }

    pub fn message(&self) -> Option<&str> {
        self.message.as_ref().map(|(text, _)| text.as_str())
    }

    pub fn spinner(&self) -> Option<char> {
        self.job.as_ref().map(|(_, at)| SPINNER[(at.elapsed().as_millis() / SPIN.as_millis()) as usize % SPINNER.len()])
    }

    /// How long the loop may block before a timed redraw; `None` blocks until input.
    pub fn wait(&self) -> Option<Duration> {
        let message = self.message.as_ref().map(|(_, at)| TTL.saturating_sub(at.elapsed()));
        message.into_iter().chain(self.job.as_ref().map(|_| SPIN)).min()
    }

    /// Starts a reindex unless one is already running.
    pub fn reindex(&mut self) {
        let (tx, rx) = mpsc::channel();
        self.job.get_or_insert_with(|| {
            thread::spawn(move || tx.send(store::reindex()));
            (rx, Instant::now())
        });
    }

    /// Expires the message and returns a finished reindex, if any.
    pub fn tick(&mut self) -> Option<Result<Index>> {
        self.message.take_if(|(_, at)| at.elapsed() >= TTL);
        let done = match self.job.as_ref()?.0.try_recv() {
            Ok(done) => done,
            Err(TryRecvError::Disconnected) => Err(anyhow!("indexer stopped unexpectedly")),
            Err(TryRecvError::Empty) => return None,
        };
        self.job = None;
        Some(done)
    }
}
