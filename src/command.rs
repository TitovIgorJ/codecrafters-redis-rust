use crate::types::BulkString;
use nom::bytes::complete::tag;
use nom::bytes::complete::take_until1;
use nom::bytes::take;
use nom::character::complete::usize;
use nom::multi::many;
use nom::{IResult, Parser};

pub(crate) fn bulk_string_array(input: &[u8]) -> IResult<&[u8], Vec<BulkString>> {
    let (input, n) = read_length(input)?;
    let (input, r) = many(0..n + 1, bulk_string).parse(input)?;
    Ok((input, r))
}

pub(crate) fn read_length(input: &[u8]) -> IResult<&[u8], usize> {
    let (input, n) = take_until1("\r\n").parse(input)?;
    let (input, _) = tag("\r\n").parse(input)?;
    let (_, n) = usize(n)?;
    Ok((input, n))
}

pub(crate) fn bulk_string(input: &[u8]) -> IResult<&[u8], BulkString> {
    let (input, _) = tag("$").parse(input)?;
    let (input, n) = read_length(input)?;
    let (input, result) = take(n).parse(input)?;
    let (input, _) = tag("\r\n").parse(input)?;
    Ok((
        input,
        BulkString {
            value: result.to_vec(),
        },
    ))
}
