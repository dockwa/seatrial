use rlua::{Lua, RegistryKey};

use std::rc::Rc;

use crate::pipe_contents::PipeContents;
use crate::pipeline::StepCompletion;
use crate::step_error::StepError;

pub fn step_function<'a>(
    idx: usize,
    fname: &str,

    // TODO: merge into a combo struct
    lua: &'a Lua,
    user_script_registry_key: &'a RegistryKey,

    last: Option<&'a PipeContents>,
) -> Result<StepCompletion, StepError> {
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

        Ok(StepCompletion::Normal {
            next_index: idx + 1,
            pipe_data: Some(PipeContents::LuaReference(Rc::new(registry_key))),
        })
    })
}
