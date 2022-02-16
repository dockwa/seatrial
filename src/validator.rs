use nanoserde::DeRon;

use std::collections::HashMap;

use crate::grunt::Grunt;
use crate::lua::stdlib::ValidationResult;
use crate::pipe_contents::PipeContents;
use crate::pipeline::action::PipelineAction as PA;
use crate::pipeline::step_handler::{
    StepCompletion, StepError, StepHandler, StepHandlerInit, StepResult,
};
use crate::pipeline::Pipeline;

#[derive(Clone, Debug, DeRon)]
pub enum Action {
    // validations of whatever the current thing in the pipe is. Asserts are generally fatal when
    // falsey, except in the context of an AnyOf or NoneOf combinator, which can "catch" the errors
    // as appropriate. WarnUnless validations are never fatal and likewise can never fail a
    // combinator
    AssertHeaderEquals(String, String),
    AssertHeaderExists(String),
    AssertStatusCode(u16),
    AssertStatusCodeInRange(u16, u16),
    WarnUnlessHeaderEquals(String, String),
    WarnUnlessHeaderExists(String),
    WarnUnlessStatusCode(u16),
    WarnUnlessStatusCodeInRange(u16, u16),

    LuaFunction(String),
}

#[derive(Debug)]
pub struct ValidatorHandler;

impl StepHandler for ValidatorHandler {
    fn new(_: &Grunt) -> StepHandlerInit<Self> {
        Ok(Self {})
    }

    fn step(&self, pl: &Pipeline, action: &PA) -> StepResult {
        match action {
            PA::Validator(validator) => match (pl.data.as_ref(), validator, pl.lua) {
                // TODO: this sucks as a UX, why aren't we providing any context as to WHY this was
                // invalid?
                (None, _, _) => Err(StepError::InvalidActionInContext),

                (Some(contents), Action::AssertHeaderExists(header_name), _) => {
                    step_assert_header_exists(header_name, contents)
                }
                (Some(contents), Action::WarnUnlessHeaderExists(header_name), _) => {
                    step_warn_unless_header_exists(header_name, contents)
                }

                (Some(contents), Action::AssertHeaderEquals(header_name, exp), _) => {
                    step_assert_header_equals(header_name, exp, contents)
                }
                (Some(contents), Action::WarnUnlessHeaderEquals(header_name, exp), _) => {
                    step_warn_unless_header_equals(header_name, exp, contents)
                }

                (Some(contents), Action::AssertStatusCode(code), _) => {
                    step_assert_status_code_eq(*code, contents)
                }
                (Some(contents), Action::WarnUnlessStatusCode(code), _) => {
                    step_warn_unless_status_code_eq(*code, contents)
                }

                (Some(contents), Action::AssertStatusCodeInRange(min_code, max_code), _) => {
                    step_assert_status_code_in_range(*min_code, *max_code, contents)
                }
                (Some(contents), Action::WarnUnlessStatusCodeInRange(min_code, max_code), _) => {
                    step_warn_unless_status_code_in_range(*min_code, *max_code, contents)
                }

                // TODO: should this put anything on the pipe?
                //
                // TODO: see if there's a sane refactor of run_user_script_function to avoid
                // double-guarding Option<PipeContents> here - for now, just discarding our current
                // knowledge since run_user_script_function needs the entire Option object anyway
                (Some(_), Action::LuaFunction(_), None) => Err(StepError::LuaNotInstantiated),
                (Some(_), Action::LuaFunction(fname), Some(lua)) => {
                    let result_rk = lua.run_user_script_function(fname, pl.data.as_ref())?;
                    lua.context(|ctx| {
                        let validation_result: ValidationResult = ctx.registry_value(&result_rk)?;
                        match validation_result {
                            ValidationResult::Ok => Ok(StepCompletion::Normal(None)),
                            ValidationResult::OkWithWarnings(warnings) => {
                                // TODO: determine if validators should put anything on the pipe
                                Ok(StepCompletion::WithWarnings(None, warnings))
                            }
                            ValidationResult::Error(err) => Err(StepError::Validation(err)),
                        }
                    })
                }
            },

            _ => unreachable!(
                "ValidatorHandler only handles Validator, Pipeline should have guarded"
            ),
        }
    }
}

fn assertion_to_warning(result: StepResult) -> StepResult {
    match result {
        // TODO: determine if validators should put anything on the pipe (and if so, there's a big
        // ol' refactor ahead)
        Err(StepError::Validation(error)) => Ok(StepCompletion::WithWarnings(None, vec![error])),
        other => other,
    }
}

#[derive(Debug)]
struct AssertionPredicateArgs<'a> {
    // allowing dead code on both of these for now because I know I plan to do rust-side
    // content-type validators in the future, and there's likely something sane to be done with
    // body eventually, I just don't know what yet
    #[allow(dead_code)]
    body: &'a Vec<u8>,
    #[allow(dead_code)]
    content_type: &'a String,

    headers: &'a HashMap<String, String>,
    status_code: u16,
}

fn simple_assertion<F>(contents: &PipeContents, failure_message: String, predicate: F) -> StepResult
where
    F: Fn(&AssertionPredicateArgs) -> bool,
{
    match contents {
        PipeContents::LuaReference(..) => Err(StepError::InvalidActionInContext),
        PipeContents::HttpResponse {
            body,
            content_type,
            headers,
            status_code,
        } => {
            if predicate(&AssertionPredicateArgs {
                body,
                content_type,
                headers,
                status_code: *status_code,
            }) {
                Ok(StepCompletion::Normal(None))
            } else {
                Err(StepError::Validation(failure_message))
            }
        }
    }
}

#[inline]
fn normalize_header_name(name: &str) -> String {
    name.trim().to_lowercase()
}

fn step_assert_header_equals(header_name: &str, exp: &str, contents: &PipeContents) -> StepResult {
    simple_assertion(
        contents,
        format!("response headers did not include \"{}\"", header_name),
        |response| {
            response
                .headers
                .get(&normalize_header_name(header_name))
                .map_or(false, |header_contents| header_contents == exp)
        },
    )
}

fn step_warn_unless_header_equals(
    header_name: &str,
    exp: &str,
    contents: &PipeContents,
) -> StepResult {
    assertion_to_warning(step_assert_header_equals(header_name, exp, contents))
}

fn step_assert_header_exists(header_name: &str, contents: &PipeContents) -> StepResult {
    simple_assertion(
        contents,
        format!("response headers did not include \"{}\"", header_name),
        |response| {
            response
                .headers
                .contains_key(&normalize_header_name(header_name))
        },
    )
}

fn step_warn_unless_header_exists(header_name: &str, contents: &PipeContents) -> StepResult {
    assertion_to_warning(step_assert_header_exists(header_name, contents))
}

fn step_assert_status_code_in_range(
    min_code: u16,
    max_code: u16,
    contents: &PipeContents,
) -> StepResult {
    simple_assertion(
        contents,
        format!("status code not in range [{}, {}]", min_code, max_code),
        |response| response.status_code >= min_code && response.status_code <= max_code,
    )
}

fn step_warn_unless_status_code_in_range(
    min_code: u16,
    max_code: u16,
    contents: &PipeContents,
) -> StepResult {
    assertion_to_warning(step_assert_status_code_in_range(
        min_code, max_code, contents,
    ))
}

fn step_assert_status_code_eq(code: u16, contents: &PipeContents) -> StepResult {
    simple_assertion(
        contents,
        format!("status code not equal to {}", code),
        |response| response.status_code == code,
    )
}

fn step_warn_unless_status_code_eq(code: u16, contents: &PipeContents) -> StepResult {
    assertion_to_warning(step_assert_status_code_eq(code, contents))
}
