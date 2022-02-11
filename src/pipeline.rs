use url::Url;

use crate::combinator::CombinatorHandler;
use crate::http::HttpHandler;
use crate::lua::LuaForPipeline;
use crate::persona::Persona;
use crate::pipe_contents::PipeContents;
use crate::validator::{Action as ValidatorAction, ValidatorHandler};

use std::collections::HashMap;

pub mod action;
use action::{ControlFlow, PipelineAction as PA, Reference};

pub mod step_handler;
use step_handler::{StepCompletion, StepError, StepHandler, StepHandlerInitError, StepResult};

#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
pub enum PipelineStepResult {
    Ok,
    OkWithWarnings(Vec<String>),
    OkWithExit,
}

impl From<StepCompletion> for PipelineStepResult {
    fn from(sc: StepCompletion) -> Self {
        match sc {
            StepCompletion::Normal(..) | StepCompletion::NoIncrement(..) => Self::Ok,
            StepCompletion::WithWarnings(_, warnings) => Self::OkWithWarnings(warnings),
            StepCompletion::WithExit => Self::OkWithExit,
        }
    }
}

#[derive(Debug)]
struct PipelineHandlers {
    pub combinator: CombinatorHandler,
    pub http: HttpHandler,
    pub validator: ValidatorHandler,
}

#[derive(Debug)]
pub struct Pipeline<'lua, 'persona, 'grunt_name, 'base_url> {
    pub data: Option<PipeContents>,
    pub persona: &'persona Persona,
    pub grunt_name: &'grunt_name str,
    pub base_url: &'base_url Url,

    // this should ideally become private, but for now,
    // crate::pipeline::action::Reference::try_stringify_potential_lua_value has us pinned into a
    // bit of a corner that needs refactoring out of
    pub lua: Option<&'lua LuaForPipeline>,

    idx: usize,
    goto_counters: HashMap<usize, usize>,
    handlers: PipelineHandlers,
}

impl<'lua, 'persona, 'grunt_name, 'base_url> Pipeline<'lua, 'persona, 'grunt_name, 'base_url> {
    pub fn new(
        grunt_name: &'grunt_name str,
        base_url: &'base_url Url,
        persona: &'persona Persona,
        lua: Option<&'lua LuaForPipeline>,
    ) -> Result<Self, StepHandlerInitError> {
        Ok(Self {
            grunt_name,
            base_url,
            persona,
            data: None,

            handlers: PipelineHandlers {
                combinator: CombinatorHandler::new(grunt_name, persona)?,
                http: HttpHandler::new(grunt_name, persona)?,
                validator: ValidatorHandler::new(grunt_name, persona)?,
            },

            lua,
            idx: 0,
            goto_counters: HashMap::with_capacity(
                persona
                    .spec
                    .sequence
                    .iter()
                    .filter(|step| matches!(step, PA::ControlFlow(ControlFlow::GoTo { .. })))
                    .count(),
            ),
        })
    }

    // this is a bit of a hack: combinators inherently need to run validators, but the only public
    // interface they have to ValidatorHandler is via pipeline, and since we're not aiming to be
    // Wordpress-esque in plugin-ability, and instead know at compile time all possible
    // interactions, we'll just brute force this one and expose a pretty wrapper method on Pipeline
    // that can be refactored out if we figure out a cleaner way to handle the "distributed
    // monolith" problem in Rust
    pub fn run_validator(&self, act: &ValidatorAction) -> Result<StepCompletion, StepError> {
        // TODO: remove need for clone by doing... anything else with lifecycle flow here
        self.handle_via(&self.handlers.validator, &PA::Validator(act.clone()))
    }

    fn step(&mut self, step: &PA) -> StepResult {
        match step {
            PA::ControlFlow(ControlFlow::GoTo { index, max_times }) => {
                self.try_goto(*index, *max_times)
            }

            PA::LuaFunction(function_name) => match self.lua {
                None => Err(StepError::LuaNotInstantiated),
                Some(lua) => {
                    let user_ret = lua.run_user_script_function(function_name, self.data.as_ref());
                    Ok(StepCompletion::Normal(
                        user_ret.map(|rk| Some(PipeContents::LuaReference(rk)))?,
                    ))
                }
            },

            PA::Reference(Reference::Value(..))
            | PA::Reference(Reference::LuaTableIndex(..))
            | PA::Reference(Reference::LuaTableValue(..))
            | PA::Reference(Reference::LuaValue) => Err(StepError::InvalidActionInContext),

            act @ PA::Http(_) => self.handle_via(&self.handlers.http, act),
            act @ PA::Combinator(_) => self.handle_via(&self.handlers.combinator, act),
            act @ PA::Validator(_) => self.handle_via(&self.handlers.validator, act),
        }
    }

    fn handle_via(&self, handler: &impl StepHandler, act: &PA) -> StepResult {
        handler.step(self, act)
    }

    fn try_goto(&mut self, index: usize, max_times: Option<usize>) -> StepResult {
        if let Some(times) = max_times {
            if times == 0 {
                // TODO: should probably warn here, or just outright disallow this (either by a
                // bounded integral type rather than usize, or by failing at lint time)
                return Ok(StepCompletion::WithExit);
            }

            match self.goto_counters.get(&index) {
                Some(rem) => {
                    if *rem == 0 {
                        return Ok(StepCompletion::WithExit);
                    }
                }
                None => {
                    self.goto_counters.insert(index, times);
                }
            };

            self.goto_counters
                .insert(index, self.goto_counters.get(&index).unwrap() - 1);
        }

        if index > self.persona.spec.sequence.len() {
            // TODO: provide details (Unclassified is deprecated)
            return Err(StepError::Unclassified);
        }

        self.idx = index;
        self.data = None;

        Ok(StepCompletion::NoIncrement(None))
    }
}

impl Iterator for Pipeline<'_, '_, '_, '_> {
    type Item = Result<PipelineStepResult, StepError>;

    fn next(&mut self) -> Option<Self::Item> {
        let ret = self
            .persona
            .spec
            .sequence
            .get(self.idx)
            .map(|step| self.step(step));

        self.data = match ret.as_ref() {
            Some(Ok(StepCompletion::Normal(data) | StepCompletion::WithWarnings(data, _))) => {
                self.idx += 1;
                data.clone()
            }

            Some(Ok(StepCompletion::NoIncrement(data))) => data.clone(),

            _ => None,
        };

        ret.map(|result| result.map(|res| res.into()))
    }
}
