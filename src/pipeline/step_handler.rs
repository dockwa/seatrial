use rlua::Error as LuaError;

use std::io::Error as IOError;

use crate::persona::Persona;
use crate::pipe_contents::PipeContents;
use crate::pipeline::action::PipelineAction;
use crate::pipeline::Pipeline;

pub type StepHandlerInit<T> = Result<T, StepHandlerInitError>;
pub type StepResult = Result<StepCompletion, StepError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StepHandlerInitError {}

#[derive(Debug)]
pub enum StepCompletion {
    Normal(Option<PipeContents>),
    NoIncrement(Option<PipeContents>),

    // TODO should this be a stronger type than just a string?
    WithWarnings(Option<PipeContents>, Vec<String>),

    WithExit,
}

#[derive(Debug)]
pub enum StepError {
    Http(ureq::Error),
    IO(IOError),
    InvalidActionInContext,

    LuaNotInstantiated,
    LuaException(LuaError),
    RefuseToStringifyComplexLuaValue,
    RefuseToStringifyNonExistantValue,
    RequestedLuaValueWhereNoneExists,

    // TODO: this is a placeholder to replace former empty struct init, remove
    Unclassified,

    UrlParsing(url::ParseError),

    Validation(String),
    ValidationSucceededUnexpectedly,
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

pub trait StepHandler
where
    Self: Sized,
{
    fn new(grunt_name: &str, persona: &Persona) -> StepHandlerInit<Self>;
    fn step(&self, pl: &Pipeline, pa: &PipelineAction) -> StepResult;
}
