use rlua::{Lua, RegistryKey};

use crate::pipe_contents::PipeContents;
use crate::pipeline::StepCompletion;
use crate::shared_lua::run_user_script_function;
use crate::step_error::StepResult;

pub fn step_function<'a>(
    idx: usize,
    fname: &str,

    // TODO: merge into a combo struct
    lua: &'a Lua,
    user_script_registry_key: &'a RegistryKey,

    last: Option<&'a PipeContents>,
) -> StepResult {
    Ok(StepCompletion::Normal {
        next_index: idx + 1,
        pipe_data: Some(PipeContents::LuaReference(run_user_script_function(
            fname,
            lua,
            user_script_registry_key,
            last,
        )?)),
    })
}
