pub struct GrepMatch {
    pub line_number: u64,
    pub text: String,
    pub match_offsets: Option<Vec<(usize, usize)>>,
}

impl GrepMatch {
    pub fn new(line_number: u64, text: String, match_offsets: Option<Vec<(usize, usize)>>) -> Self {
        Self {
            line_number,
            text,
            match_offsets,
        }
    }
}
