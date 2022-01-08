use crate::pipe_contents::PipeContents;

#[derive(Debug)]
pub enum StepCompletion {
    Normal {
        next_index: usize,
        pipe_data: Option<PipeContents>,
    },
    WithWarnings {
        next_index: usize,
        pipe_data: Option<PipeContents>,
    },
    WithExit,
}
