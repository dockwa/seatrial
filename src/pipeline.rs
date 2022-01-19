use rlua::Error as LuaError;

#[derive(Debug)]
pub enum PipeContents {
    HttpResponse(ureq::Response),
    LuaReference(String),
}

impl PipeContents {
    pub fn to_lua_string(&self) -> String {
        match self {
            PipeContents::HttpResponse(_) => unimplemented!(),
            PipeContents::LuaReference(lref) => lref.into(),
        }
    }
}

#[derive(Debug)]
pub enum StepCompletion {
    Normal {
        next_index: usize,
        pipe_data: Option<PipeContents>,
    },
    WithWarnings {
        next_index: usize,
        pipe_data: Option<PipeContents>,
    },
    WithExit,
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
