use nom::{IResult, alt, char, character::streaming::digit1, do_parse, named, opt, tag};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkEvent {
    Connected { link_id: usize },
    Closed { link_id: usize },
    DataAvailable { link_id: usize, size: usize },
}

fn atoi(digits: &[u8]) -> Option<usize> {
    let mut num: usize = 0;
    let len = digits.len();

    for (i, digit) in digits.iter().enumerate() {
        let digit = (*digit as char).to_digit(10)? as usize;
        let mut exp = 1;
        for _ in 0..(len - i - 1) {
            exp *= 10;
        }
        num += exp * digit;
    }
    Some(num)
}

fn parse_usize(input: &[u8]) -> IResult<&[u8], usize> {
    let (input, digits) = digit1(input)?;
    let num = atoi(digits).unwrap();
    IResult::Ok((input, num))
}

#[rustfmt::skip]
named!(
    crlf,
    tag!("\r\n")
);

named!(
    connected<NetworkEvent>,
    do_parse!(
        opt!( crlf ) >>
        link_id: parse_usize >>
        tag!(",CONNECT") >>
        crlf >>
        (
            NetworkEvent::Connected { link_id, }
        )
    )
);

named!(
    closed<NetworkEvent>,
    do_parse!(
        opt!( crlf ) >>
        link_id: parse_usize >>
        tag!(",CLOSED") >>
        crlf >>
        (
            NetworkEvent::Closed { link_id, }
        )
    )
);

named!(
    data_available<NetworkEvent>,
    do_parse!(
        opt!( crlf ) >>
        tag!( "+IPD,") >>
        link_id: parse_usize >>
        char!(',') >>
        size: parse_usize >>
        char!(':') >>
        opt!( crlf ) >>
        (
            NetworkEvent::DataAvailable { link_id, size }
        )
    )
);

named!(
    parse<NetworkEvent>,
    alt!(
        connected
        | closed
        | data_available
    )
);

impl NetworkEvent {
    pub fn parse(input: &[u8]) -> Option<(&[u8], Self)> {
        parse(input).ok()
    }
}
