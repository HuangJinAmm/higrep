use anyhow::Result;
use std::sync::{mpsc, Arc};

use super::{sink::MatchesSink, SearchConfig};
use crate::file_entry::FileEntry;
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
    inner: Arc<SearcherImpl>,
    tx: mpsc::Sender<Event>,
}

impl Searcher {
    pub(crate) fn new(config: SearchConfig, tx: mpsc::Sender<Event>) -> Self {
        Self {
            inner: Arc::new(SearcherImpl::new(config)),
            tx,
        }
    }

    pub(crate) fn search(&self) {
        let local_self = self.inner.clone();
        let tx_local = self.tx.clone();
        let _ = std::thread::spawn(move || {
            if local_self.run(tx_local.clone()).is_err() {
                tx_local.send(Event::Error).ok();
            }

            tx_local.send(Event::SearchingFinished).ok();
        });
    }
}

struct SearcherImpl {
    config: SearchConfig,
}

impl SearcherImpl {
    fn new(config: SearchConfig) -> Self {
        Self { config }
    }

    fn run(&self, tx: mpsc::Sender<Event>) -> Result<()> {
        let grep_searcher = SearcherBuilder::new()
            .binary_detection(BinaryDetection::quit(b'\x00'))
            .line_terminator(LineTerminator::byte(b'\n'))
            .line_number(true)
            .multi_line(false)
            .after_context(3)
            .before_context(4)
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
