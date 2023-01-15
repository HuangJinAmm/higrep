use crate::grep_match::GrepMatch;

pub enum EntryType {
    Header(String),
    Match(u64, String, Option<Vec<(usize, usize)>>),
}

pub struct FileEntry(Vec<EntryType>);

impl FileEntry {
    pub fn new(name: String, matches: Vec<GrepMatch>) -> Self {
        Self(
            std::iter::once(EntryType::Header(name))
                .chain(
                    matches
                        .into_iter()
                        .map(|m| {

                            let text = vec!["-";100].join("-");
                            if m.text == text {
                                EntryType::Header(text)
                            } else {
                                EntryType::Match(m.line_number, m.text, m.match_offsets)
                            }
                        }),
                )
                .collect(),
        )
    }

    pub fn get_matches_count(&self) -> usize {
        self.0
            .iter()
            .filter(|&e| matches!(e, EntryType::Match(_, _, _)))
            .count()
    }

    pub fn get_entries(self) -> Vec<EntryType> {
        self.0
    }
}
