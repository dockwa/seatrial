use nanoserde::DeRon;

use crate::grunt::Grunt;
use crate::pipeline::action::PipelineAction;
use crate::pipeline::step_handler::{
    StepCompletion, StepError, StepHandler, StepHandlerInit, StepResult,
};
use crate::pipeline::Pipeline;
use crate::validator::Action as ValidatorAction;

// allow "all have same postfix" to pass since these names pass directly through to the config file
// (thus become a ux implication)
#[allow(clippy::enum_variant_names)]
#[derive(Clone, Debug, DeRon)]
pub enum Action {
    AllOf(Vec<ValidatorAction>),
    AnyOf(Vec<ValidatorAction>),
    NoneOf(Vec<ValidatorAction>),
}

#[derive(Debug)]
pub struct CombinatorHandler;

impl StepHandler for CombinatorHandler {
    fn new(_: &Grunt) -> StepHandlerInit<Self> {
        Ok(Self {})
    }

    fn step(&self, pl: &Pipeline, action: &PipelineAction) -> StepResult {
        match action {
            PipelineAction::Combinator(combi) => match combi {
                Action::AllOf(validators) => self.all_of(pl, validators),
                Action::AnyOf(validators) => self.any_of(pl, validators),
                Action::NoneOf(validators) => {
                    match self.any_of(pl, validators) {
                        // TODO plumb details up the chain
                        Ok(_) => Err(StepError::ValidationSucceededUnexpectedly),

                        // TODO should this be a NormalWithWarnings? there were failures, we just
                        // explicitly ignored them...
                        Err(_) => Ok(StepCompletion::Normal(None)),
                    }
                }
            },

            _ => unreachable!(
                "CombinatorHandler only handles Combinator, Pipeline should have guarded"
            ),
        }
    }
}

// TODO should combinators put anything in the output pipe??
impl CombinatorHandler {
    fn all_of(&self, pl: &Pipeline, it: &[ValidatorAction]) -> StepResult {
        let mut combined_warnings: Vec<String> = Vec::with_capacity(it.len());

        for validator in it {
            match pl.run_validator(validator)? {
                StepCompletion::Normal(_) | StepCompletion::NoIncrement(_) => {}
                StepCompletion::WithWarnings(_, warnings) => combined_warnings.extend(warnings),
                StepCompletion::WithExit => {
                    unimplemented!(
                        "combinator members requesting a pipeline exit is not implemented"
                    )
                }
            }
        }

        if combined_warnings.is_empty() {
            Ok(StepCompletion::Normal(None))
        } else {
            Ok(StepCompletion::WithWarnings(None, combined_warnings))
        }
    }

    fn any_of(&self, pl: &Pipeline, it: &[ValidatorAction]) -> StepResult {
        for validator in it {
            match pl.run_validator(validator) {
                ret @ Ok(StepCompletion::Normal(_) | StepCompletion::NoIncrement(_)) => return ret,
                ret @ Ok(StepCompletion::WithWarnings(_, _)) => return ret,

                Ok(StepCompletion::WithExit) => unreachable!("validators should not be able to kill the program via OkWithExit, only by errors"),

                Err(_) =>  {},
            };
        }

        Err(StepError::Validation(
            "no validators in combinator succeeded".into(),
        ))
    }
}
