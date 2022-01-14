use rlua::{Error as LuaError, Lua};

use crate::pipeline::{PipeContents, StepCompletion, StepError};

pub fn step_function(idx: usize, fname: &str, lua: &mut Lua) -> Result<StepCompletion, StepError> {
    lua.context(|ctx| {
        let ref_name = format!("pipeline_call_{}", fname);
        let globals = ctx.globals();

        eprintln!("running lua function {}", fname);

        ctx.load(&format!(
            "{} = user_script[\"{}\"]() -- TODO: add last",
            ref_name, fname
        ))
        .set_name(&format!("pipeline action<{}>", fname))?
        .exec()?;

        Ok(StepCompletion::Success {
            next_index: idx + 1,
            pipe_data: Some(PipeContents::LuaReference(ref_name)),
        })
    })
    .map_err(|err: LuaError| err.into())
}
