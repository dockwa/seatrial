use nanoserde::DeRon;

use std::collections::HashMap;

use crate::config_duration::ConfigDuration;

pub type ConfigActionMap = HashMap<String, PipelineAction>;

#[derive(Clone, Debug, DeRon)]
pub enum PipelineAction {
    GoTo {
        index: usize,
        max_times: Option<usize>,
    },
    // this is mostly used for URL params, since those _can_ come from Lua, and thus have to be a
    // PipelineAction member
    Value(String),

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

    // basic logic. rust doesn't allow something like
    // All(AssertStatusCode|AssertStatusCodeInRange), so instead, **any** PipelineAction is a valid
    // member of a combinator for now, which is less than ideal ergonomically to say the least
    AllOf(Vec<PipelineAction>),
    AnyOf(Vec<PipelineAction>),
    NoneOf(Vec<PipelineAction>),

    // the "Here Be Dragons" section, for when dynamism is absolutely needed: an escape hatch to
    // Lua. TODO: document the Lua APIs and semantics...
    LuaFunction(String),
    LuaValue,
    LuaTableIndex(usize),
    LuaTableValue(String),
}
