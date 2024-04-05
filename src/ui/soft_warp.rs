use unicode_width::UnicodeWidthChar;

#[derive(Debug, PartialEq, Eq , Ord)]
pub enum SplitPosType {
    Crlf(usize),
    MatchStart(usize),
    MatchEnd(usize),
}

impl PartialOrd for SplitPosType {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let l = match self {
            SplitPosType::Crlf(x) => x,
            SplitPosType::MatchStart(x) => x,
            SplitPosType::MatchEnd(x) => x,
        };
        let r = match other {
            SplitPosType::Crlf(x) => x,
            SplitPosType::MatchStart(x) => x,
            SplitPosType::MatchEnd(x) => x,
        };
        Some(l.cmp(r))
    }
}

pub struct SoftWrapper {
    pub positions: Vec<SplitPosType>,
}

impl SoftWrapper {
    pub fn new(max_width: usize, matches_offsets: &Vec<(usize, usize)>, text: &str) -> Self {
        let mut positions = Vec::new();
        if text.is_empty() {
            return Self { positions };
        }
        let uni_chars = text.chars();

        let mut current_len = 0;
        let mut byte_pos = 0;
        for c in uni_chars {
            if let Some(c_width) = UnicodeWidthChar::width(c) {
                current_len += c_width;
                if current_len > max_width {
                    positions.push(SplitPosType::Crlf(byte_pos));
                    current_len = c_width;
                }
            }
            byte_pos += c.len_utf8();
        }

        for (start, end) in matches_offsets {
            positions.push(SplitPosType::MatchStart(start.to_owned()));
            positions.push(SplitPosType::MatchEnd(end.to_owned()));
        }
        positions.push(SplitPosType::Crlf(text.len()));
        positions.sort();

        Self { positions }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ord() {
        let mut spts = vec![
            SplitPosType::Crlf(4),
            SplitPosType::MatchStart(2),
            SplitPosType::MatchEnd(3),
            SplitPosType::Crlf(8),
            SplitPosType::Crlf(12),
            SplitPosType::MatchStart(10),
            SplitPosType::MatchEnd(11),
        ];

        spts.sort();

        println!("{spts:#?}");
    }

    #[test]
    fn test_hanzi() {
        let s = "ab从啊解决\r\ndd法大师傅a".to_owned();
        let offsets = vec![];
        let soft = SoftWrapper::new(3, &offsets, &s);

        let mut c = 0;
        for spt in soft.positions {
            match spt {
                SplitPosType::Crlf(x) => {
                    println!("CR|{}", &s[c..x]);
                    c = x;
                }
                SplitPosType::MatchStart(x) => {
                    println!("MS|{}", &s[c..x]);
                    c = x;
                }
                SplitPosType::MatchEnd(x) => {
                    println!("ME|{}", &s[c..x]);
                    c = x;
                }
            }
        }
    }
}
