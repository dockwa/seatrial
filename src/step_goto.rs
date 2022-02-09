use crate::persona::Persona;
use crate::pipeline::StepCompletion;
use crate::step_error::{StepError, StepResult};

pub fn step(desired_index: usize, persona: &Persona) -> StepResult {
    if desired_index > persona.spec.pipeline.len() {
        // TODO: provide details (expand enum to allow)
        return Err(StepError::Unclassified);
    }

    Ok(StepCompletion::Normal {
        next_index: desired_index,
        pipe_data: None,
    })
}
