use std::collections::HashMap;

use crate::pipe_contents::PipeContents;
use crate::pipeline::StepCompletion;
use crate::pipeline_action::Validator;
use crate::lua::stdlib::ValidationResult;
use crate::lua::LuaForPipeline;
use crate::step_error::{StepError, StepResult};

pub fn step<'a>(
    idx: usize,
    it: &Validator,
    lua: &LuaForPipeline,
    last: Option<&'a PipeContents>,
) -> StepResult {
    match (last, it) {
        // TODO: this sucks as a UX, why aren't we providing any context as to WHY this was
        // invalid?
        (None, _) => Err(StepError::InvalidActionInContext),

        (Some(contents), Validator::AssertHeaderExists(header_name)) => {
            step_assert_header_exists(idx, header_name, contents)
        }
        (Some(contents), Validator::WarnUnlessHeaderExists(header_name)) => {
            step_warn_unless_header_exists(idx, header_name, contents)
        }

        (Some(contents), Validator::AssertHeaderEquals(header_name, exp)) => {
            step_assert_header_equals(idx, header_name, exp, contents)
        }
        (Some(contents), Validator::WarnUnlessHeaderEquals(header_name, exp)) => {
            step_warn_unless_header_equals(idx, header_name, exp, contents)
        }

        (Some(contents), Validator::AssertStatusCode(code)) => {
            step_assert_status_code_eq(idx, *code, contents)
        }
        (Some(contents), Validator::WarnUnlessStatusCode(code)) => {
            step_warn_unless_status_code_eq(idx, *code, contents)
        }

        (Some(contents), Validator::AssertStatusCodeInRange(min_code, max_code)) => {
            step_assert_status_code_in_range(idx, *min_code, *max_code, contents)
        }
        (Some(contents), Validator::WarnUnlessStatusCodeInRange(min_code, max_code)) => {
            step_warn_unless_status_code_in_range(idx, *min_code, *max_code, contents)
        }

        // TODO: should this put anything on the pipe?
        //
        // TODO: see if there's a sane refactor of run_user_script_function to avoid
        // double-guarding Option<PipeContents> here - for now, just discarding our current
        // knowledge since run_user_script_function needs the entire Option object anyway
        (Some(_), Validator::LuaFunction(fname)) => {
            let result_rk = lua.run_user_script_function(fname, last)?;
            lua.context(|ctx| {
                let validation_result: ValidationResult = ctx.registry_value(&result_rk)?;
                match validation_result {
                    ValidationResult::Ok => Ok(StepCompletion::Normal {
                        next_index: idx + 1,
                        pipe_data: None,
                    }),
                    ValidationResult::OkWithWarnings(warnings) => {
                        Ok(StepCompletion::WithWarnings {
                            next_index: idx + 1,
                            pipe_data: None,
                            warnings,
                        })
                    }
                    ValidationResult::Error(err) => Err(StepError::Validation(err)),
                }
            })
        }
    }
}

// TODO I've been thinking this for ages and here's where I'm finally writing it down: I'm tired of
// toting idx around. this, &'a Lua, user_script_registry_key, and various other bits I keep
// plumbing around are inherently part of the grunt's state and should be encapsulated in some
// object these methods just take mutable reference to
fn assertion_to_warning(result: StepResult, idx: usize, contents: &PipeContents) -> StepResult {
    match result {
        Err(StepError::Validation(error)) => Ok(StepCompletion::WithWarnings {
            next_index: idx + 1,
            pipe_data: Some(contents.clone()),
            warnings: vec![error],
        }),
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

fn simple_assertion<F>(
    idx: usize,
    contents: &PipeContents,
    failure_message: String,
    predicate: F,
) -> StepResult
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
                Ok(StepCompletion::Normal {
                    next_index: idx + 1,
                    pipe_data: Some(contents.clone()),
                })
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

fn step_assert_header_equals(
    idx: usize,
    header_name: &str,
    exp: &str,
    contents: &PipeContents,
) -> StepResult {
    simple_assertion(
        idx,
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
    idx: usize,
    header_name: &str,
    exp: &str,
    contents: &PipeContents,
) -> StepResult {
    assertion_to_warning(
        step_assert_header_equals(idx, header_name, exp, contents),
        idx,
        contents,
    )
}

fn step_assert_header_exists(idx: usize, header_name: &str, contents: &PipeContents) -> StepResult {
    simple_assertion(
        idx,
        contents,
        format!("response headers did not include \"{}\"", header_name),
        |response| {
            response
                .headers
                .contains_key(&normalize_header_name(header_name))
        },
    )
}

fn step_warn_unless_header_exists(
    idx: usize,
    header_name: &str,
    contents: &PipeContents,
) -> StepResult {
    assertion_to_warning(
        step_assert_header_exists(idx, header_name, contents),
        idx,
        contents,
    )
}

fn step_assert_status_code_in_range(
    idx: usize,

    min_code: u16,
    max_code: u16,

    contents: &PipeContents,
) -> StepResult {
    simple_assertion(
        idx,
        contents,
        format!("status code not in range [{}, {}]", min_code, max_code),
        |response| response.status_code >= min_code && response.status_code <= max_code,
    )
}

fn step_warn_unless_status_code_in_range(
    idx: usize,

    min_code: u16,
    max_code: u16,

    contents: &PipeContents,
) -> StepResult {
    assertion_to_warning(
        step_assert_status_code_in_range(idx, min_code, max_code, contents),
        idx,
        contents,
    )
}

fn step_assert_status_code_eq(idx: usize, code: u16, contents: &PipeContents) -> StepResult {
    simple_assertion(
        idx,
        contents,
        format!("status code not equal to {}", code),
        |response| response.status_code == code,
    )
}

fn step_warn_unless_status_code_eq(idx: usize, code: u16, contents: &PipeContents) -> StepResult {
    assertion_to_warning(
        step_assert_status_code_eq(idx, code, contents),
        idx,
        contents,
    )
}
