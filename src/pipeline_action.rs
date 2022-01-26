use nanoserde::DeRon;
use rlua::Lua;

use std::collections::HashMap;

use crate::config_duration::ConfigDuration;
use crate::pipe_contents::PipeContents as PC;
use crate::shared_lua::try_stringify_lua_value;
use crate::step_error::StepError;

pub type ConfigActionMap = HashMap<String, Reference>;

// allow "all have same postfix" to pass since these names pass directly through to the config file
// (thus become a ux implication)
#[allow(clippy::enum_variant_names)]
#[derive(Clone, Debug, DeRon)]
pub enum Combinator {
    AllOf(Vec<Validator>),
    AnyOf(Vec<Validator>),
    NoneOf(Vec<Validator>),
}

#[derive(Clone, Debug, DeRon)]
pub enum ControlFlow {
    GoTo {
        index: usize,
        max_times: Option<usize>,
    },
}

#[derive(Clone, Debug, DeRon)]
pub enum Http {
    // http verbs. this section could be fewer LOC with macros eg
    // https://stackoverflow.com/a/37007315/17630058, but (1) this is still manageable (there's
    // only a few HTTP verbs), and (2) rust macros are cryptic enough to a passer-by that if we're
    // going to introduce them and their mental overhead to this codebase (other than depending on
    // a few from crates), we should have a strong reason (and perhaps multiple usecases).

    // TODO: figure out what, if anything, are appropriate guardrails for a PATCH verb
    Delete {
        url: String,
        headers: Option<ConfigActionMap>,
        params: Option<ConfigActionMap>,
        timeout: Option<ConfigDuration>,
    },
    Get {
        url: String,
        headers: Option<ConfigActionMap>,
        params: Option<ConfigActionMap>,
        timeout: Option<ConfigDuration>,
    },
    Head {
        url: String,
        headers: Option<ConfigActionMap>,
        params: Option<ConfigActionMap>,
        timeout: Option<ConfigDuration>,
    },
    Post {
        url: String,
        headers: Option<ConfigActionMap>,
        params: Option<ConfigActionMap>,
        timeout: Option<ConfigDuration>,
    },
    Put {
        url: String,
        headers: Option<ConfigActionMap>,
        params: Option<ConfigActionMap>,
        timeout: Option<ConfigDuration>,
    },
}

#[derive(Clone, Debug, DeRon)]
pub enum Reference {
    // this is mostly used for URL params, since those _can_ come from Lua, and thus have to be a
    // PipelineAction member
    Value(String),

    LuaValue,
    LuaTableIndex(usize),
    LuaTableValue(String),
}

impl Reference {
    pub fn try_into_string_given_pipe_data(
        &self,
        lua: &Lua,
        pipe_data: Option<&PC>,
    ) -> Result<String, StepError> {
        match self {
            Reference::Value(it) => Ok(it.clone()),
            Reference::LuaValue => match pipe_data {
                None => Err(StepError::RequestedLuaValueWhereNoneExists),
                // TODO: as with Unclassified itself, change this
                Some(PC::HttpResponse { .. }) => Err(StepError::Unclassified),
                Some(PC::LuaReference(rkey)) => lua.context(|ctx| {
                    try_stringify_lua_value(ctx.registry_value::<rlua::Value>(rkey))
                }),
            },
            Reference::LuaTableIndex(idx) => match pipe_data {
                None => Err(StepError::RequestedLuaValueWhereNoneExists),
                // TODO: as with Unclassified itself, change this
                Some(PC::HttpResponse { .. }) => Err(StepError::Unclassified),
                Some(PC::LuaReference(rkey)) => lua.context(|ctx| {
                    try_stringify_lua_value(ctx.registry_value::<rlua::Table>(rkey)?.get(*idx))
                }),
            },
            Reference::LuaTableValue(key) => match pipe_data {
                None => Err(StepError::RequestedLuaValueWhereNoneExists),
                // TODO: as with Unclassified itself, change this
                Some(PC::HttpResponse { .. }) => Err(StepError::Unclassified),
                Some(PC::LuaReference(rkey)) => lua.context(|ctx| {
                    try_stringify_lua_value(
                        ctx.registry_value::<rlua::Table>(rkey)?.get(key.clone()),
                    )
                }),
            },
        }
    }
}

#[derive(Clone, Debug, DeRon)]
pub enum Validator {
    // validations of whatever the current thing in the pipe is. Asserts are generally fatal when
    // falsey, except in the context of an AnyOf or NoneOf combinator, which can "catch" the errors
    // as appropriate. WarnUnless validations are never fatal and likewise can never fail a
    // combinator
    AssertHeaderExists(String),
    AssertStatusCode(u16),
    AssertStatusCodeInRange(u16, u16),
    WarnUnlessHeaderExists(String),
    WarnUnlessStatusCode(u16),
    WarnUnlessStatusCodeInRange(u16, u16),

    LuaFunction(String),
}

#[derive(Clone, Debug, DeRon)]
pub enum PipelineAction {
    Combinator(Combinator),
    ControlFlow(ControlFlow),
    Http(Http),
    LuaFunction(String),
    Reference(Reference),
    Validator(Validator),
}
