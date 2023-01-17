use anyhow::Result;
use std::{
    borrow::BorrowMut,
    cell::{Cell, RefCell},
    sync::{mpsc, Arc, RwLock},
};

use super::{sink::MatchesSink, SearchConfig};
use crate::{file_entry::FileEntry, ui::cmd_parse::SearchCmd};
use grep::{
    matcher::LineTerminator,
    regex::RegexMatcherBuilder,
    searcher::{BinaryDetection, SearcherBuilder},
};
use ignore::WalkBuilder;

pub(crate) enum Event {
    NewEntry(FileEntry),
    SearchingFinished,
    Error,
}

pub(crate) struct Searcher {
    inner: Arc<RwLock<SearcherImpl>>,
    tx: mpsc::Sender<Event>,
}

impl Searcher {
    pub(crate) fn new(config: SearchConfig, tx: mpsc::Sender<Event>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(SearcherImpl::new(config))),
            tx,
        }
    }

    pub(crate) fn search(&self) {
        let local_self_clone = self.inner.clone();
        let tx_local = self.tx.clone();
        let _ = std::thread::spawn(move || {
            if let Ok(local_self_th) = local_self_clone.read() {
                if local_self_th.run(tx_local.clone()).is_err() {
                    tx_local.send(Event::Error).ok();
                }

                tx_local.send(Event::SearchingFinished).ok();
            }
        });
    }

    pub(crate) fn update_cmd(&mut self, cmd: SearchCmd) {
        let mut lock = self.inner.write().unwrap();
        lock.update_cmd(cmd)
    }
}

struct SearcherImpl {
    config: SearchConfig,
}

impl SearcherImpl {
    fn new(config: SearchConfig) -> Self {
        Self { config }
    }

    fn update_cmd(&mut self, cmd: SearchCmd) {
        self.config.update_from(cmd);
    }

    fn run(&self, tx: mpsc::Sender<Event>) -> Result<()> {
        let grep_searcher = SearcherBuilder::new()
            .binary_detection(BinaryDetection::quit(b'\x00'))
            .line_terminator(LineTerminator::byte(b'\n'))
            .line_number(true)
            .multi_line(false)
            .after_context(self.config.after_context)
            .before_context(self.config.before_context)
            .build();

        let matcher = RegexMatcherBuilder::new()
            .line_terminator(Some(b'\n'))
            .case_insensitive(self.config.case_insensitive)
            .case_smart(self.config.case_smart)
            .build(&self.config.pattern)?;
        let mut builder = WalkBuilder::new(&self.config.path);

        let walk_parallel = builder
            .overrides(self.config.overrides.clone())
            .types(self.config.types.clone())
            .hidden(!self.config.search_hidden)
            .build_parallel();
        walk_parallel.run(move || {
            let tx = tx.clone();
            let matcher = matcher.clone();
            let mut grep_searcher = grep_searcher.clone();

            Box::new(move |result| {
                let dir_entry = match result {
                    Ok(entry) => {
                        if !entry.file_type().map_or(false, |ft| ft.is_file()) {
                            return ignore::WalkState::Continue;
                        }
                        entry
                    }
                    Err(_) => return ignore::WalkState::Continue,
                };
                let mut matches_in_entry = Vec::new();
                let sr = MatchesSink::new(&matcher, &mut matches_in_entry);
                grep_searcher
                    .search_path(&matcher, dir_entry.path(), sr)
                    .ok();

                if !matches_in_entry.is_empty() {
                    tx.send(Event::NewEntry(FileEntry::new(
                        dir_entry.path().to_string_lossy().into_owned(),
                        matches_in_entry,
                    )))
                    .ok();
                }

                ignore::WalkState::Continue
            })
        });

        Ok(())
    }
}
