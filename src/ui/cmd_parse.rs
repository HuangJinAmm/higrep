use nom::branch::alt;
use nom::bytes::complete::{tag, take_until, take_while};
use nom::character::complete::{char};
use nom::multi::{many_m_n};
use nom::sequence::{pair, preceded};
use nom::IResult;

pub struct SearchCmd {
    pub pattern: String,
    pub before_context: usize,
    pub after_context: usize,
}

impl SearchCmd {
    pub fn parse(cmd: &String) -> Option<Self> {
        if cmd.contains("--") {
            let Ok(rp) = parttern_parser(cmd) else {return None};
            let around = around_parser(rp.0).unwrap_or(("", (0, 0)));
            let pat = rp.1.trim().to_owned();
            Some(Self {
                pattern: pat,
                before_context: around.1 .0 as usize,
                after_context: around.1 .1 as usize,
            })
        } else {
            if cmd.is_empty() {
                None
            } else {
                Some(Self {
                    pattern: cmd.to_owned(),
                    before_context: 0,
                    after_context: 0,
                })
            }
        }
    }
}

fn to_u16(input: &str) -> Result<u16, std::num::ParseIntError> {
    u16::from_str_radix(input, 10)
}

fn take_num_str(input: &str) -> IResult<&str, &str> {
    take_while(|c: char| c.is_digit(10))(input)
}

fn take_str(input: &str) -> IResult<&str, &str> {
    take_while(|c: char| c.is_ascii_alphanumeric())(input)
}

fn after_num(input: &str) -> IResult<&str, (char, &str)> {
    pair(char('a'), take_num_str)(input)
}
fn before_num(input: &str) -> IResult<&str, (char, &str)> {
    pair(char('b'), take_num_str)(input)
}

fn a_or_b(input: &str) -> IResult<&str, (char, &str)> {
    alt((after_num, before_num))(input)
}

fn ab_num_parser(input: &str) -> IResult<&str, Vec<(char, &str)>> {
    many_m_n(0, 2, a_or_b)(input)
}

fn around_parser(input: &str) -> IResult<&str, (u16, u16)> {
    let res = preceded(tag("--"), take_str)(input)?;
    if let Ok(a_or_b) = ab_num_parser(res.1) {
        let v_ab = a_or_b.1;
        match v_ab[..] {
            [] => {
                let ab = to_u16(res.1).unwrap_or_default();
                Ok((res.0, (ab, ab)))
            }
            [('a', a), ('b', b), ..] => {
                let an = to_u16(a).unwrap_or_default();
                let bn = to_u16(b).unwrap_or_default();
                Ok((res.0, (an, bn)))
            }
            [('b', b), ('a', a), ..] => {
                let an = to_u16(a).unwrap_or_default();
                let bn = to_u16(b).unwrap_or_default();
                Ok((res.0, (an, bn)))
            }
            [('b', b), ..] => {
                let bn = to_u16(b).unwrap_or_default();
                Ok((res.0, (0, bn)))
            }
            [('a', a), ..] => {
                let an = to_u16(a).unwrap_or_default();
                Ok((res.0, (an, 0)))
            }
            _ => Ok((res.0, (0, 0))),
        }
    } else {
        let ab = to_u16(res.1).unwrap_or_default();
        Ok((res.0, (ab, ab)))
    }
}

fn parttern_parser(input: &str) -> IResult<&str, &str> {
    take_until("--")(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser() {
        let input = "--";
        let res = around_parser(input).unwrap();
        println!("{:#?}", res);
        // assert_eq!(res.1,123);
        // assert_eq!(res.0,&b"-b"[..]);
    }

    #[test]
    fn test_parttern_parser() {
        let res = parttern_parser("input a test - a \\ --a10b5");
        println!("{:#?}", res);
    }

    #[test]
    fn test_ab() {
        let res = ab_num_parser("a100");
        println!("{:#?}", res);
        let res = ab_num_parser("b100a50");
        println!("{:#?}", res);
    }
}
