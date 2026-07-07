use clap::{
    builder::TypedValueParser,
    error::{ContextKind, ContextValue, Error, ErrorKind},
};
use regex::Regex;

const REGEX_APPSTREAM: &str = r#"^(?:appstream)(?:://)((?:[[[:alpha:]][0-9]\-_]+\.)*(?:[[[:alpha:]][0-9]\-_]+))(?:\?alt=((?:(?:(?:[[[:alpha:]][0-9]\-_]+\.)*(?:[[[:alpha:]][0-9]\-_]+)),)*(?:(?:[[[:alpha:]][0-9]\-_]+\.)*(?:[[[:alpha:]][0-9]\-_]+))))?"#;
const REGEX_NIXPKG: &str =
    r#"^(?:nixpkg)(?:://)((?:[[[:alpha:]][0-9]\-_]+\.)*(?:[[[:alpha:]][0-9]\-_]+))$"#;

#[derive(Debug, Clone)]
pub enum Scheme {
    AppStream {
        id: String,
        alt: Option<Vec<String>>,
    },
    NixPkg(String),
}

#[derive(Clone)]
pub struct SchemeParser;

impl TypedValueParser for SchemeParser {
    type Value = Scheme;

    fn parse_ref(
        &self,
        _cmd: &clap::Command,
        _arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        if let Some(value) = value.to_str() {
            parse_scheme(value)
        } else {
            let mut err = clap::Error::new(ErrorKind::MissingRequiredArgument);
            err.insert(ContextKind::InvalidArg, ContextValue::None);

            Err(err)
        }
    }
}

fn parse_appstream(captures: regex::Captures) -> Result<Scheme, Error> {
    let id = captures.get(1).map(|m| m.as_str().to_string());
    let alt = captures.get(2).map(|m| {
        m.as_str()
            .split(',')
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
    });

    if let Some(id) = id {
        Ok(Scheme::AppStream { id, alt })
    } else {
        Err(Error::new(ErrorKind::InvalidValue))
    }
}

fn parse_nixpkg(captures: regex::Captures) -> Result<Scheme, Error> {
    let pkg = captures.get(1).map(|m| m.as_str().to_string());
    if let Some(pkg) = pkg {
        Ok(Scheme::NixPkg(pkg))
    } else {
        Err(Error::new(ErrorKind::InvalidValue))
    }
}

fn parse_scheme(arg: &str) -> Result<Scheme, Error> {
    let re_appstream = Regex::new(REGEX_APPSTREAM).unwrap();
    let re_nixpkg = Regex::new(REGEX_NIXPKG).unwrap();

    if let Some(captures) = re_appstream.captures(arg) {
        parse_appstream(captures)
    } else if let Some(captures) = re_nixpkg.captures(arg) {
        parse_nixpkg(captures)
    } else {
        Err(Error::new(ErrorKind::UnknownArgument))
    }
}
