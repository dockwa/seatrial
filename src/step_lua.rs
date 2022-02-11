use crate::pipe_contents::PipeContents;
use crate::pipeline::StepCompletion;
use crate::lua::LuaForPipeline;
use crate::step_error::StepResult;

pub fn step_function<'a>(
    idx: usize,
    fname: &str,
    lua: &LuaForPipeline,
    last: Option<&'a PipeContents>,
) -> StepResult {
    Ok(StepCompletion::Normal {
        next_index: idx + 1,
        pipe_data: Some(PipeContents::LuaReference(
            lua.run_user_script_function(fname, last)?,
        )),
    })
}
