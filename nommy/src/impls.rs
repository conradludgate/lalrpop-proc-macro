use crate::{Buffer, Cursor, Parse, Peek, Process};
use std::{error::Error, fmt::Debug};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("error should not have occured. This is probably a bug with nommy")]
pub struct NeverError;

impl<P: Peek<T>, T> Peek<T> for Option<P> {
    fn peek(input: &mut Cursor<impl Iterator<Item = T>>) -> bool {
        let mut cursor = input.cursor();

        let skip = if P::peek(&mut cursor) {
            cursor.close()
        } else {
            0
        };

        input.skip(skip);
        // Option should always return true for peek
        true
    }
}

/// Define Parse for Option<P>.
/// Result is None if parsing P fails
/// Otherwise, result is Some(p)
impl<P: Parse<T>, T> Parse<T> for Option<P> {
    type Error = NeverError;
    fn parse(input: &mut Buffer<impl Iterator<Item = T>>) -> Result<Self, Self::Error> {
        if P::peek(&mut input.cursor()) {
            Ok(Some(
                P::parse(input).expect("peek succeeded but parse failed"),
            ))
        } else {
            Ok(None)
        }
    }
}

impl<P: Process> Process for Option<P> {
    type Output = Option<P::Output>;
    fn process(self) -> Self::Output {
        self.map(P::process)
    }
}

impl<P: Peek<T>, T> Peek<T> for Vec<P> {
    fn peek(input: &mut Cursor<impl Iterator<Item = T>>) -> bool {
        loop {
            let mut cursor = input.cursor();
            if !P::peek(&mut cursor) {
                break;
            }
            let skip = cursor.close();
            input.skip(skip);
        }
        true
    }
}

/// Define Parse for Vec<P>.
/// Repeatedly attempt to parse P,
/// Result is all successful attempts
impl<P: Parse<T>, T> Parse<T> for Vec<P> {
    type Error = NeverError;
    fn parse(input: &mut Buffer<impl Iterator<Item = T>>) -> Result<Self, Self::Error> {
        let mut output = vec![];
        while P::peek(&mut input.cursor()) {
            output.push(P::parse(input).expect("peek succeeded but parse failed"));
        }

        Ok(output)
    }
}

impl<P: Process> Process for Vec<P> {
    type Output = Vec<P::Output>;
    fn process(self) -> Self::Output {
        self.into_iter().map(P::process).collect()
    }
}

/// Vec1 is similar to `Vec` but implements `Parse` such that it will error if it fails to parse at least once
#[derive(Debug, Clone, PartialEq)]
pub struct Vec1<P>(Vec<P>);

impl<P> AsRef<Vec<P>> for Vec1<P> {
    fn as_ref(&self) -> &Vec<P> {
        &self.0
    }
}

impl<P> AsMut<Vec<P>> for Vec1<P> {
    fn as_mut(&mut self) -> &mut Vec<P> {
        &mut self.0
    }
}

impl<P> Vec1<P> {
    pub fn into_inner(self) -> Vec<P> {
        self.0
    }
}

impl<P: Peek<T>, T> Peek<T> for Vec1<P> {
    fn peek(input: &mut Cursor<impl Iterator<Item = T>>) -> bool {
        if !P::peek(input) {
            return false;
        }

        loop {
            let mut cursor = input.cursor();
            if !P::peek(&mut cursor) {
                break;
            }
            let skip = cursor.close();
            input.skip(skip);
        }

        true
    }
}

/// Define Parse for Vec1<P>.
/// Repeatedly attempt to parse P,
/// Result is all successful attempts
impl<P: Parse<T>, T> Parse<T> for Vec1<P> {
    type Error = P::Error;
    fn parse(input: &mut Buffer<impl Iterator<Item = T>>) -> Result<Self, Self::Error> {
        let mut output = vec![P::parse(input)?];
        while P::peek(&mut input.cursor()) {
            output.push(P::parse(input).expect("peek succeeded but parse failed"));
        }

        Ok(Vec1(output))
    }
}

impl<P: Process> Process for Vec1<P> {
    type Output = Vec<P::Output>;
    fn process(self) -> Self::Output {
        self.0.into_iter().map(P::process).collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrefixedBy<Prefix, P> {
    pub prefix: Prefix,
    pub parsed: P,
}

#[derive(Debug, Clone, PartialEq, Error)]
pub enum PrefixedByParseError<PrefixParseError, ParseError>
where
    PrefixParseError: Error,
    ParseError: Error,
{
    #[error("could not parse prefix: {0}")]
    Prefix(Box<PrefixParseError>),
    #[error("could not parse body: {0}")]
    Parsed(Box<ParseError>),
}

impl<Prefix: Peek<T>, P: Peek<T>, T> Peek<T> for PrefixedBy<Prefix, P> {
    fn peek(input: &mut Cursor<impl Iterator<Item = T>>) -> bool {
        Prefix::peek(input) && P::peek(input)
    }
}

/// Define Parse for PrefixedBy<P>.
/// Parse Prefix then parse P
impl<Prefix: Parse<T>, P: Parse<T>, T> Parse<T> for PrefixedBy<Prefix, P> {
    type Error = PrefixedByParseError<Prefix::Error, P::Error>;
    fn parse(input: &mut Buffer<impl Iterator<Item = T>>) -> Result<Self, Self::Error> {
        Ok(PrefixedBy {
            prefix: Prefix::parse(input)
                .map_err(|err| PrefixedByParseError::Prefix(Box::new(err)))?,
            parsed: P::parse(input).map_err(|err| PrefixedByParseError::Parsed(Box::new(err)))?,
        })
    }
}

impl<Prefix, P: Process> Process for PrefixedBy<Prefix, P> {
    type Output = P::Output;
    fn process(self) -> Self::Output {
        self.parsed.process()
    }
}

#[cfg(test)]
mod tests {
    use crate::{parse, token::*};

    use super::Vec1;

    #[test]
    fn option() {
        let res: Result<Option<Dot>, _> = parse(".".chars());
        assert_eq!(res.unwrap(), Some(Dot))
    }

    #[test]
    fn option_none() {
        let res: Result<Option<Dot>, _> = parse("".chars());
        assert_eq!(res.unwrap(), None)
    }

    #[test]
    fn sequence() {
        let res: Result<Vec<Dot>, _> = parse("...".chars());
        assert_eq!(res.unwrap(), vec![Dot, Dot, Dot])
    }

    #[test]
    fn sequence_none() {
        let res: Result<Vec<Dot>, _> = parse("".chars());
        assert_eq!(res.unwrap(), vec![])
    }

    #[test]
    fn sequence_at_least_one() {
        let res: Result<Vec1<Dot>, _> = parse("...".chars());
        assert_eq!(res.unwrap().into_inner(), vec![Dot, Dot, Dot])
    }

    #[test]
    fn sequence_at_least_one_but_none() {
        let res: Result<Vec1<Dot>, _> = parse("".chars());
        assert_eq!(format!("{}", res.unwrap_err()), "error parsing tag `.`");
    }
}
