use std::collections::HashMap;

use crate::persona::Persona;
use crate::pipeline::{StepCompletion, StepError};

pub fn step(
    desired_index: usize,
    max_times: Option<usize>,
    persona: &Persona,
    goto_counters: &mut HashMap<usize, usize>,
) -> Result<StepCompletion, StepError> {
    if desired_index > persona.spec.pipeline.len() {
        // TODO: provide details (expand enum to allow)
        return Err(StepError::Unclassified);
    }

    Ok(StepCompletion::Success {
        next_index: desired_index,
        pipe_data: None,
    })
}
