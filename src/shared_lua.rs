use rlua::{Error as LuaError, Value as LuaValue};

use crate::step_error::StepError;

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
        Ok(LuaValue::Thread { .. } | LuaValue::Error {..}) => Err(StepError::RefuseToStringifyComplexLuaValue),
        Err(err) => Err(err.into()),
    }
}
