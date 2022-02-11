use rlua::{Lua, RegistryKey};

use crate::pipe_contents::PipeContents;
use crate::pipeline::StepCompletion;
use crate::pipeline_action::{Combinator, Validator};
use crate::step_error::{StepError, StepResult};
use crate::step_validator::step as do_step_validator;

pub fn step<'a>(
    idx: usize,
    it: &Combinator,

    // TODO: merge into a combo struct
    lua: &'a Lua,
    user_script_registry_key: &'a RegistryKey,

    last: Option<&'a PipeContents>,
) -> StepResult {
    match it {
        Combinator::AllOf(validators) => {
            all_of(idx, lua, user_script_registry_key, last, validators)
        }
        Combinator::AnyOf(validators) => {
            any_of(idx, lua, user_script_registry_key, last, validators)
        }
        Combinator::NoneOf(validators) => {
            match any_of(idx, lua, user_script_registry_key, last, validators) {
                // TODO plumb details up the chain
                Ok(_) => Err(StepError::ValidationSucceededUnexpectedly),

                // TODO should this be a NormalWithWarnings? there were failures, we just
                // explicitly ignored them...
                Err(_) => Ok(StepCompletion::Normal {
                    next_index: idx + 1,
                    pipe_data: None,
                }),
            }
        }
    }
}

fn all_of<'a>(
    idx: usize,

    // TODO: merge into a combo struct
    lua: &'a Lua,
    user_script_registry_key: &'a RegistryKey,

    last: Option<&'a PipeContents>,

    it: &[Validator],
) -> StepResult {
    let mut combined_warnings: Vec<String> = Vec::with_capacity(it.len());

    for validator in it {
        match do_step_validator(idx, validator, lua, user_script_registry_key, last)? {
            StepCompletion::Normal { .. } => {}
            StepCompletion::WithWarnings { warnings, .. } => combined_warnings.extend(warnings),
            StepCompletion::WithExit { .. } => {
                unimplemented!("combinator members requesting a pipeline exit is not implemented")
            }
        }
    }

    if combined_warnings.is_empty() {
        Ok(StepCompletion::Normal {
            next_index: idx + 1,
            // TODO should validators put anything in the output pipe??
            pipe_data: None,
        })
    } else {
        Ok(StepCompletion::WithWarnings {
            next_index: idx + 1,
            // TODO should validators put anything in the output pipe??
            pipe_data: None,
            warnings: combined_warnings,
        })
    }
}

fn any_of<'a>(
    idx: usize,

    // TODO: merge into a combo struct
    lua: &'a Lua,
    user_script_registry_key: &'a RegistryKey,

    last: Option<&'a PipeContents>,

    it: &[Validator],
) -> StepResult {
    for validator in it {
        if let result @ Ok(_) =
            do_step_validator(idx, validator, lua, user_script_registry_key, last)
        {
            return result;
        }
    }

    Err(StepError::Validation(
        "no validators in combinator succeeded".into(),
    ))
}
