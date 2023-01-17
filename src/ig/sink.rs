use crate::grep_match::GrepMatch;
use grep::{
    matcher::Matcher,
    searcher::{Searcher, Sink, SinkMatch},
};

pub(crate) struct MatchesSink<'a, M>
where
    M: Matcher,
{
    matcher: M,
    matches_in_entry: &'a mut Vec<GrepMatch>,
}

impl<'a, M> MatchesSink<'a, M>
where
    M: Matcher,
{
    pub(crate) fn new(matcher: M, matches_in_entry: &'a mut Vec<GrepMatch>) -> Self {
        Self {
            matcher,
            matches_in_entry,
        }
    }
}

impl<'a, M> Sink for MatchesSink<'a, M>
where
    M: Matcher,
{
    type Error = std::io::Error;

    fn matched(&mut self, _: &Searcher, sink_match: &SinkMatch) -> Result<bool, std::io::Error> {
        let line_number = sink_match
            .line_number()
            .ok_or(std::io::ErrorKind::InvalidData)?;
        let text = std::str::from_utf8(sink_match.bytes());

        let mut offsets = vec![];
        self.matcher
            .find_iter(sink_match.bytes(), |m| {
                offsets.push((m.start(), m.end()));
                true
            })
            .ok();

        if let Ok(t) = text {
            self.matches_in_entry
                .push(GrepMatch::new(line_number, t.into(), Some(offsets)));
        };

        Ok(true)
    }

    fn context(
        &mut self,
        _searcher: &Searcher,
        context: &grep::searcher::SinkContext<'_>,
    ) -> Result<bool, Self::Error> {
        let line_num = context
            .line_number()
            .ok_or(std::io::ErrorKind::InvalidData)?;
        let text = std::str::from_utf8(context.bytes());
        if let Ok(t) = text {
            self.matches_in_entry
                .push(GrepMatch::new(line_num, t.into(), None));
        }
        Ok(true)
    }

    fn context_break(&mut self, _searcher: &Searcher) -> Result<bool, Self::Error> {
        let line_num = 0;
        let text = vec!["-"; 100].join("-");
        self.matches_in_entry
            .push(GrepMatch::new(line_num, text, None));
        Ok(true)
    }
}
