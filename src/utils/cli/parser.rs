use winnow::{
    Parser, Result,
    combinator::{alt, opt, preceded, separated},
    error::{ContextError, ParseError},
    token::{one_of, take_while},
};

use super::scheme::Scheme;

fn name_first(input: &mut &str) -> Result<char> {
    one_of(('a'..='z', 'A'..='Z', '_')).parse_next(input)
}

fn name_part<'s>(input: &mut &'s str) -> Result<&'s str> {
    take_while(0.., ('a'..='z', 'A'..='Z', '0'..='9', '_', '-')).parse_next(input)
}

fn name_rest(input: &mut &str) -> Result<String> {
    let parts: Vec<&str> = separated(0.., name_part, ".").parse_next(input)?;
    Ok(parts.join("."))
}

fn parse_name(input: &mut &str) -> Result<String> {
    (name_first, name_rest)
        .take()
        .parse_next(input)
        .map(|s| s.to_string())
}

fn parse_nixpkg_prefix<'s>(input: &mut &'s str) -> Result<&'s str> {
    "nixpkg://".parse_next(input)
}

fn parse_nixpkg_package(input: &mut &str) -> Result<Scheme> {
    let name = preceded(parse_nixpkg_prefix, parse_name).parse_next(input)?;

    Ok(Scheme::NixPkg(name))
}

fn parse_appstream_prefix<'s>(input: &mut &'s str) -> Result<&'s str> {
    "appstream://".parse_next(input)
}

fn parse_appstream_alts(input: &mut &str) -> Result<Vec<String>> {
    "?alt=".parse_next(input)?;
    separated(0.., parse_name, ",").parse_next(input)
}

fn parse_appstream_package(input: &mut &str) -> Result<Scheme> {
    let (name, alts) = (
        preceded(parse_appstream_prefix, parse_name),
        opt(parse_appstream_alts),
    )
        .parse_next(input)?;

    Ok(Scheme::AppStream {
        id: name,
        alt: alts,
    })
}

pub fn parse_scheme<'s>(
    input: &mut &'s str,
) -> std::result::Result<Scheme, ParseError<&'s str, ContextError>> {
    alt((parse_nixpkg_package, parse_appstream_package)).parse(input)
}
