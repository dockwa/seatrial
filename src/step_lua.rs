use rlua::{Error as LuaError, Lua};

use crate::pipeline::{PipeContents, StepCompletion, StepError};

pub fn step_function(
    idx: usize,
    fname: &str,
    lua: &mut Lua,
    last: Option<&PipeContents>,
) -> Result<StepCompletion, StepError> {
    lua.context(|ctx| {
        let ref_name = format!("pipeline_call_{}", fname);

        let last_arg = match last {
            Some(contents) => contents.to_lua_string(),
            None => "nil".into(),
        };

        ctx.load(&format!(
            "{} = user_script[\"{}\"]({})",
            ref_name, fname, last_arg,
        ))
        .set_name(&format!("pipeline action<{}>", fname))?
        .exec()?;

        Ok(StepCompletion::Normal {
            next_index: idx + 1,
            pipe_data: Some(PipeContents::LuaReference(ref_name)),
        })
    })
    .map_err(|err: LuaError| err.into())
}
