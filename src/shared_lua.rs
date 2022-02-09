use rlua::{Error as LuaError, Lua, RegistryKey, Value as LuaValue};

use std::rc::Rc;

use crate::pipe_contents::PipeContents;
use crate::step_error::StepError;

pub mod stdlib;
pub use stdlib::attach_seatrial_stdlib;

pub fn try_stringify_lua_value(it: Result<LuaValue, LuaError>) -> Result<String, StepError> {
    match it {
        Ok(LuaValue::Nil) => Err(StepError::RequestedLuaValueWhereNoneExists),
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

pub fn run_user_script_function<'a>(
    fname: &str,

    // TODO: merge into a combo struct
    lua: &'a Lua,
    user_script_registry_key: &'a RegistryKey,

    last: Option<&'a PipeContents>,
) -> Result<Rc<RegistryKey>, StepError> {
    lua.context(|ctx| {
        let lua_func = ctx
            .registry_value::<rlua::Table>(user_script_registry_key)?
            .get::<_, rlua::Function>(fname)?;
        let script_arg = match last {
            Some(lval) => match lval.to_lua(lua)? {
                Some(rkey) => ctx.registry_value::<rlua::Value>(&rkey)?,
                None => rlua::Nil,
            },
            None => rlua::Nil,
        };
        let result = lua_func.call::<rlua::Value, rlua::Value>(script_arg)?;
        let registry_key = ctx.create_registry_value(result)?;
        Ok(Rc::new(registry_key))
    })
}
