use rlua::Error as LuaError;

#[derive(Debug)]
pub enum PipeContents {
    HttpResponse(ureq::Response),
    LuaReference(String),
}

#[derive(Debug)]
pub enum StepCompletion {
    Success {
        next_index: usize,
        pipe_data: Option<PipeContents>,
    },
    SuccessWithWarnings {
        next_index: usize,
        pipe_data: Option<PipeContents>,
    },
}

#[derive(Debug)]
pub enum StepError {
    // TODO: this is a placeholder to replace former empty struct init, remove
    Unclassified,
    InvalidActionInContext,
    LuaException(LuaError),
    UrlParsing(url::ParseError),
    Http(ureq::Error),
}

impl From<LuaError> for StepError {
    fn from(src: LuaError) -> Self {
        Self::LuaException(src)
    }
}
