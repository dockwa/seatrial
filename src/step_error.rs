use rlua::Error as LuaError;

use std::io::Error as IOError;

#[derive(Debug)]
pub enum StepError {
    Http(ureq::Error),
    IO(IOError),
    InvalidActionInContext,
    LuaException(LuaError),
    RefuseToStringifyComplexLuaValue,
    RequestedLuaValueWhereNoneExists,

    // TODO: this is a placeholder to replace former empty struct init, remove
    Unclassified,

    UrlParsing(url::ParseError),
}

impl From<IOError> for StepError {
    fn from(err: IOError) -> Self {
        Self::IO(err)
    }
}

impl From<LuaError> for StepError {
    fn from(src: LuaError) -> Self {
        Self::LuaException(src)
    }
}
