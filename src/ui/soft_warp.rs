
#[derive(Debug,PartialEq,Eq,Ord)]
pub enum SplitPosType {
    Crlf(usize),
    MatchStart(usize),
    MatchEnd(usize)
}

impl PartialOrd for SplitPosType {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let l = match self {
            SplitPosType::Crlf(x) => x,
            SplitPosType::MatchStart(x) => x,
            SplitPosType::MatchEnd(x) =>x,
        };
        let r = match other {
            SplitPosType::Crlf(x) => x,
            SplitPosType::MatchStart(x) => x,
            SplitPosType::MatchEnd(x) =>x,
        };
        Some(l.cmp(r))
    }
}

pub struct SoftWrapper {
    pub positions: Vec<SplitPosType>,
}

impl SoftWrapper {
    
    pub fn new(max_width:usize,matches_offsets:&Option<Vec<(usize,usize)>>,max_len:usize) -> Self {
        let mut positions = Vec::new();
        let count = max_len/max_width;
        for i in 1..=count {
            positions.push(SplitPosType::Crlf(i*max_width));
        }
        if let Some(off_sets) = matches_offsets {
            for (start,end) in off_sets{
                positions.push(SplitPosType::MatchStart(start.to_owned()));
                positions.push(SplitPosType::MatchEnd(end.to_owned()));
            }
        }
        positions.push(SplitPosType::Crlf(max_len));
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

        println!("{:#?}",spts);
    }
}