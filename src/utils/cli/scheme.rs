use clap::{
    builder::TypedValueParser,
    error::{ContextKind, ContextValue, ErrorKind},
};

use super::parser::parse_scheme;

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
        if let Some(mut input) = value.to_str() {
            let scheme = parse_scheme(&mut input);
            scheme.map_err(|_| {
                let mut err = clap::Error::new(ErrorKind::UnknownArgument);
                err.insert(ContextKind::InvalidArg, ContextValue::None);

                err
            })
        } else {
            let mut err = clap::Error::new(ErrorKind::MissingRequiredArgument);
            err.insert(ContextKind::InvalidArg, ContextValue::None);

            Err(err)
        }
    }
}
