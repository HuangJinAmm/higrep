use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref AB_RE: Regex = Regex::new("--([ab]?\\d+)([ab]\\d+)?").unwrap();
    static ref CMD_RE_Q: Regex =
        Regex::new("\"([^\"]+)\"([^\"-]*)(--([ab]?\\d+)([ab]\\d+)?)?").unwrap();
    static ref CMD_RE: Regex = Regex::new("([^\" ]+)([^\"-]*)(--([ab]?\\d+)([ab]\\d+)?)?").unwrap();
}

#[derive(Debug, PartialEq)]
pub struct SearchCmd {
    pub pattern: String,
    pub golb: Option<Vec<String>>,
    pub before_context: usize,
    pub after_context: usize,
}

impl SearchCmd {
    pub fn parse(cmd: &String) -> Option<Self> {
        if cmd.contains("--") || cmd.contains(' ') {
            let caps;
            if cmd.starts_with('\"') {
                if CMD_RE_Q.is_match(cmd) {
                    let Some(caps_re) = CMD_RE_Q.captures(cmd) else {return None};
                    caps = caps_re;
                } else {
                    return None;
                }
            } else if CMD_RE.is_match(cmd) {
                let Some(caps_re) = CMD_RE.captures(cmd) else {return None};
                caps = caps_re;
            } else {
                return None;
            }
            let Some(pat) = caps.get(1) else {return None};
            let mut glob_vec = None;
            if let Some(glob) = caps.get(2) {
                glob_vec = Some(Vec::new());
                let gstr = glob.as_str().trim().split(' ');
                for g in gstr {
                    if g.is_empty() {
                        continue;
                    }
                    glob_vec.as_mut().unwrap().push(g.to_owned());
                }
            }
            let mut a = 0;
            let mut b = 0;
            if let Some(ar) =caps.get(3) {
                let arstr = ar.as_str();
                if let Some(ab) = parse_ab(arstr) {
                    a = ab.0;
                    b = ab.1;
                }
            }
            return Some(Self {
                pattern: pat.as_str().to_owned(),
                golb: glob_vec,
                before_context: b,
                after_context: a,
            });
        } else if cmd.is_empty() {
            None
        } else {
            Some(Self {
                pattern: cmd.to_owned(),
                before_context: 0,
                after_context: 0,
                golb: None,
            })
        }
    }
}

fn parse_ab(input: &str) -> Option<(usize, usize)> {
    let ms = AB_RE.captures(input).unwrap();
    let mut a = 0;
    let mut b = 0;
    if let Some(g1) = ms.get(1) {
        let mstr = g1.as_str();
        if mstr.starts_with('a') {
            if let Some(num) = mstr.strip_prefix('a') {
                a = usize::from_str_radix(num, 10).unwrap_or_default();
            }
        } else if mstr.starts_with('b') {
            if let Some(num) = mstr.strip_prefix('b') {
                b = usize::from_str_radix(num, 10).unwrap_or_default();
            }
        } else {
            a = usize::from_str_radix(mstr, 10).unwrap_or_default();
            b = a;
        }
    }

    if let Some(g1) = ms.get(2) {
        let mstr = g1.as_str();
        if mstr.starts_with('a') {
            if let Some(num) = mstr.strip_prefix('a') {
                a = usize::from_str_radix(num, 10).unwrap_or_default();
            }
        } else if mstr.starts_with('b') {
            if let Some(num) = mstr.strip_prefix('b') {
                b = usize::from_str_radix(num, 10).unwrap_or_default();
            }
        } else {
            a = usize::from_str_radix(mstr, 10).unwrap_or_default();
            b = a;
        }
    }

    Some((a, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_re() {
        let text = "\"传输速度\" *.rs *.json --a100b23";
        let ms = CMD_RE_Q.captures(text).unwrap();
        ms.iter().enumerate().for_each(|s| {
            if let Some(re) = s.1 {
                println!("{}--{}",s.0,re.as_str());
            } else {
                println!("{}--None",s.0);
            }
        });
        println!("----------------");
        let text = "传输速度 *.rs *.json --a100b23";
        let ms = CMD_RE.captures(text).unwrap();
        ms.iter().enumerate().for_each(|s| {
            if let Some(re) = s.1 {
                println!("{}--{}",s.0,re.as_str());
            } else {
                println!("{}--None",s.0);
            }
        });
    }

    #[test]
    fn test_ab() {
        let text = "\"传输速度\" *.rs *.json --a100b23";
        let ms = CMD_RE_Q.captures(text).unwrap();
        let ab = ms.get(3).unwrap().as_str();
        let ab = parse_ab(ab).unwrap();
        println!("{},{}", ab.0, ab.1);
    }

    #[test]
    fn test_cmd_1() {
        let text = "\"传输速度\" *.rs *.json --a100b23".to_owned();
        let cmd = SearchCmd::parse(&text).unwrap();
        let sc = SearchCmd {
            pattern: "传输速度".to_owned(),
            golb: Some(vec!["*.rs".to_owned(), "*.json".to_owned()]),
            before_context: 23,
            after_context: 100,
        };
        assert_eq!(cmd, sc);
    }
    #[test]
    fn test_cmd_2() {
        let sc = SearchCmd {
            pattern: "传输-速度".to_owned(),
            golb: Some(vec!["*.rs".to_owned(), "*.json".to_owned()]),
            before_context: 23,
            after_context: 100,
        };
        let text = "传输-速度 *.rs *.json --a100b23".to_owned();
        let cmd = SearchCmd::parse(&text).unwrap();
        assert_eq!(cmd, sc);
    }

    #[test]
    fn test_cmd_3() {
        let sc = SearchCmd {
            pattern: "传输速度".to_owned(),
            golb: Some(vec!["*.rs".to_owned(), "*.json".to_owned()]),
            before_context: 100,
            after_context: 100,
        };
        let text = "传输速度 *.rs *.json --100".to_owned();
        let cmd = SearchCmd::parse(&text).unwrap();
        assert_eq!(cmd, sc);
    }

    #[test]
    fn test_cmd_4() {
        let sc = SearchCmd {
            pattern: "传输速度".to_owned(),
            golb: Some(vec!["*.rs".to_owned(), "*.json".to_owned()]),
            before_context: 0,
            after_context: 0,
        };
        let text = "传输速度 *.rs *.json ".to_owned();
        let cmd = SearchCmd::parse(&text).unwrap();
        assert_eq!(cmd, sc);
    }

    #[test]
    fn test_cmd_5() {
        let sc = SearchCmd {
            pattern: "传输 速度".to_owned(),
            golb: Some(vec!["*.rs".to_owned(), "*.json".to_owned()]),
            before_context: 0,
            after_context: 0,
        };
        let text = "\"传输 速度\" *.rs *.json ".to_owned();
        let cmd = SearchCmd::parse(&text).unwrap();
        assert_eq!(cmd, sc);
    }

    #[test]
    fn test_cmd_6() {
        let sc = SearchCmd {
            pattern: "传输-- 速度".to_owned(),
            golb: Some(Vec::new()),
            before_context: 0,
            after_context: 0,
        };
        let text = "\"传输-- 速度\"".to_owned();
        let cmd = SearchCmd::parse(&text).unwrap();
        assert_eq!(cmd, sc);
    }

    #[test]
    fn test_cmd_7() {
        let sc = SearchCmd {
            pattern: "传输--速度".to_owned(),
            golb: Some(Vec::new()),
            before_context: 0,
            after_context: 0,
        };
        let text = "传输--速度".to_owned();
        let cmd = SearchCmd::parse(&text).unwrap();
        assert_eq!(cmd, sc);
    }

    #[test]
    fn test_cmd_8() {
        let sc = SearchCmd {
            pattern: "传输-- 速度".to_owned(),
            golb: Some(Vec::new()),
            before_context: 22,
            after_context: 10,
        };
        let text = "\"传输-- 速度\"--b22a10".to_owned();
        let cmd = SearchCmd::parse(&text).unwrap();
        assert_eq!(cmd, sc);
    }

    #[test]
    fn test_cmd_9() {
        let sc = SearchCmd {
            pattern: "传输--速度".to_owned(),
            golb: Some(Vec::new()),
            before_context: 100,
            after_context: 100,
        };
        let text = "传输--速度 --100".to_owned();
        let cmd = SearchCmd::parse(&text).unwrap();
        assert_eq!(cmd, sc);
    }
}
