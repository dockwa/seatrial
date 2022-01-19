use crate::persona::Persona;
use crate::pipeline::{StepCompletion, StepError};

pub fn step(desired_index: usize, persona: &Persona) -> Result<StepCompletion, StepError> {
    if desired_index > persona.spec.pipeline.len() {
        // TODO: provide details (expand enum to allow)
        return Err(StepError::Unclassified);
    }

    Ok(StepCompletion::Normal {
        next_index: desired_index,
        pipe_data: None,
    })
}
