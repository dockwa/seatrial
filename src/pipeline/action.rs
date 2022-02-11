use nanoserde::DeRon;
use rlua::{Error as LuaError, Value as LuaValue};

use std::collections::HashMap;

use crate::lua::LuaForPipeline;
use crate::http::Action as HttpAction;
use crate::pipe_contents::PipeContents as PC;
use crate::pipeline::step_handler::StepError;
use crate::combinator::Action as CombinatorAction;
use crate::validator::Action as ValidatorAction;

pub type ConfigActionMap = HashMap<String, Reference>;

#[derive(Clone, Debug, DeRon)]
pub enum ControlFlow {
    GoTo {
        index: usize,
        max_times: Option<usize>,
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
        lua: Option<&LuaForPipeline>,
        pipe_data: Option<&PC>,
    ) -> Result<String, StepError> {
        match (self, lua) {
            (Reference::Value(it), _) => Ok(it.clone()),

            (_, None) => Err(StepError::LuaNotInstantiated),

            (Reference::LuaValue, Some(lua)) => match pipe_data {
                None => Err(StepError::RequestedLuaValueWhereNoneExists),
                // TODO: as with Unclassified itself, change this
                Some(PC::HttpResponse { .. }) => Err(StepError::Unclassified),
                Some(PC::LuaReference(rkey)) => lua.context(|ctx| {
                    self.try_stringify_potential_lua_value(ctx.registry_value::<rlua::Value>(rkey))
                }),
            },

            (Reference::LuaTableIndex(idx), Some(lua)) => match pipe_data {
                None => Err(StepError::RequestedLuaValueWhereNoneExists),
                // TODO: as with Unclassified itself, change this
                Some(PC::HttpResponse { .. }) => Err(StepError::Unclassified),
                Some(PC::LuaReference(rkey)) => lua.context(|ctx| {
                    self.try_stringify_potential_lua_value(
                        ctx.registry_value::<rlua::Table>(rkey)?.get(*idx),
                    )
                }),
            },

            (Reference::LuaTableValue(key), Some(lua)) => match pipe_data {
                None => Err(StepError::RequestedLuaValueWhereNoneExists),
                // TODO: as with Unclassified itself, change this
                Some(PC::HttpResponse { .. }) => Err(StepError::Unclassified),
                Some(PC::LuaReference(rkey)) => lua.context(|ctx| {
                    self.try_stringify_potential_lua_value(
                        ctx.registry_value::<rlua::Table>(rkey)?.get(key.clone()),
                    )
                }),
            },
        }
    }

    fn try_stringify_potential_lua_value(
        &self,
        it: Result<LuaValue, LuaError>,
    ) -> Result<String, StepError> {
        match it {
            Ok(LuaValue::Nil) => Err(StepError::RefuseToStringifyNonExistantValue),
            Ok(LuaValue::Boolean(val)) => Ok(val.to_string()),
            Ok(LuaValue::Integer(val)) => Ok(val.to_string()),
            Ok(LuaValue::Number(val)) => Ok(val.to_string()),
            Ok(LuaValue::String(val)) => Ok(val.to_str()?.into()),
            Ok(
                LuaValue::Table(..)
                | LuaValue::Function(..)
                | LuaValue::UserData(..)
                | LuaValue::LightUserData(..),
            ) => Err(StepError::RefuseToStringifyComplexLuaValue),
            Ok(LuaValue::Thread { .. } | LuaValue::Error { .. }) => {
                Err(StepError::RefuseToStringifyComplexLuaValue)
            }
            Err(err) => Err(err.into()),
        }
    }
}

#[derive(Clone, Debug, DeRon)]
pub enum PipelineAction {
    Combinator(CombinatorAction),
    ControlFlow(ControlFlow),
    Http(HttpAction),
    LuaFunction(String),
    Reference(Reference),
    Validator(ValidatorAction),
}
