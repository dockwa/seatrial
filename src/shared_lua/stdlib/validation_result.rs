// TODO: this file's public Lua API needs documented in the seatrial.lua(3) manual or a subpage
// thereof

use rlua::{
    Context, Error as LuaError, FromLua, Lua, Result as LuaResult, ToLua, Value as LuaValue,
    Variadic,
};

// values stored as markers in the Lua tables that get passed around as validation result "enums",
// or a Lua approximation of the Rust concept thereof
const VALIDATION_RESULT_CODE_KEY: &'static str = "_validation_result_code";
const VALIDATION_RESULT_WARNINGS_KEY: &'static str = "_validation_result_warnings";
const VALIDATION_RESULT_ERROR_KEY: &'static str = "_validation_result_error";
const VALIDATION_RESULT_OK_CODE: i8 = 1;
const VALIDATION_RESULT_OK_WITH_WARNINGS_CODE: i8 = 2;
const VALIDATION_RESULT_ERROR_CODE: i8 = 3;
const VALIDATION_RESULT_TABLE_IDX_BOUNDS: (i8, i8) =
    (VALIDATION_RESULT_OK_CODE, VALIDATION_RESULT_ERROR_CODE); // keep up to date
const VALIDATION_RESULT_MISSING_WARNING_ARG_MSG: &'static str =
    "expected at least one warning string, got none";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValidationResult {
    Ok,

    // TODO: rather than just shuttling strings around, is there a function in the lua VM context
    // to dump a stack trace of where the warning initiated? the lua API might look as such:
    //
    // ```lua
    // function myvalidator()
    //     return ValidationResult.OkWithWarnings(
    //         ValidationWarning("validator did nothing, which I guess is a form of being okay")
    //     )
    // end
    // ```
    OkWithWarnings(Vec<String>),

    // TODO: ditto from OkWithWarnings
    Error(String),
}

impl<'lua> FromLua<'lua> for ValidationResult {
    fn from_lua(lval: LuaValue<'lua>, _: Context<'lua>) -> LuaResult<Self> {
        match lval {
            LuaValue::Table(table) => match table.get(VALIDATION_RESULT_CODE_KEY)? {
                LuaValue::Number(code) => validation_result_by_i8(code as i8, table),
                LuaValue::Integer(code) => validation_result_by_i8(code as i8, table),

                other => Err(LuaError::RuntimeError(
                    format!("expected number at table key {}, got {:?}", VALIDATION_RESULT_CODE_KEY, other)
                ))
            }

            other => Err(LuaError::RuntimeError(
                format!("only tables (generally constructed by seatrial itself) can become ValidationResults, not {:?}", other),
            )),
        }
    }
}

fn validation_result_by_i8(code: i8, table: rlua::Table) -> LuaResult<ValidationResult> {
    match code {
        VALIDATION_RESULT_OK_CODE => Ok(ValidationResult::Ok),

        VALIDATION_RESULT_OK_WITH_WARNINGS_CODE => {
            match table.get(VALIDATION_RESULT_WARNINGS_KEY)? {
                LuaValue::Table(lua_warnings) => {
                    // I expect this will generally be called with one error, so for
                    // conservation of RAM, we'll allocate just enough room for one string
                    // for now. rust will re-allocate as necessary under the hood should
                    // this assumption be proven wrong, and we'll take a slight perf hit.
                    // whatever.
                    let mut warnings: Vec<String> = Vec::with_capacity(1);

                    for warning_val in lua_warnings.sequence_values::<String>() {
                        warnings.push(warning_val?);
                    }

                    if warnings.is_empty() {
                        return Err(LuaError::RuntimeError(
                            VALIDATION_RESULT_MISSING_WARNING_ARG_MSG.into(),
                        ));
                    }

                    Ok(ValidationResult::OkWithWarnings(warnings))
                }
                other => Err(LuaError::RuntimeError(format!(
                    "expected table at table key {}, got {:?}",
                    VALIDATION_RESULT_OK_WITH_WARNINGS_CODE, other
                ))),
            }
        }

        VALIDATION_RESULT_ERROR_CODE => match table.get(VALIDATION_RESULT_ERROR_KEY)? {
            LuaValue::String(error) => Ok(ValidationResult::Error(error.to_str()?.into())),
            other => Err(LuaError::RuntimeError(format!(
                "expected table at table key {}, got {:?}",
                VALIDATION_RESULT_OK_WITH_WARNINGS_CODE, other
            ))),
        },

        other => Err(LuaError::RuntimeError(format!(
            "expected in-bounds number ({}-{}) at table key {}, got {}",
            VALIDATION_RESULT_TABLE_IDX_BOUNDS.0,
            VALIDATION_RESULT_TABLE_IDX_BOUNDS.1,
            VALIDATION_RESULT_CODE_KEY,
            other,
        ))),
    }
}

impl<'lua> ToLua<'lua> for ValidationResult {
    fn to_lua(self, ctx: Context<'lua>) -> LuaResult<LuaValue<'lua>> {
        let container = ctx.create_table()?;

        Ok(match self {
            ValidationResult::Ok => {
                container.set(VALIDATION_RESULT_CODE_KEY, VALIDATION_RESULT_OK_CODE)?;
                LuaValue::Table(container)
            }

            ValidationResult::OkWithWarnings(warnings) => {
                container.set(
                    VALIDATION_RESULT_CODE_KEY,
                    VALIDATION_RESULT_OK_WITH_WARNINGS_CODE,
                )?;
                container.set(VALIDATION_RESULT_WARNINGS_KEY, warnings)?;
                LuaValue::Table(container)
            }

            ValidationResult::Error(err) => {
                container.set(VALIDATION_RESULT_CODE_KEY, VALIDATION_RESULT_ERROR_CODE)?;
                container.set(VALIDATION_RESULT_ERROR_KEY, err)?;
                LuaValue::Table(container)
            }
        })
    }
}

pub fn attach_validationresult<'a>(lua: &'a Lua) -> LuaResult<()> {
    lua.context(|ctx| {
        let globals = ctx.globals();

        // these all return Ok because we don't actually want the lua execution context to raise an
        // error, we want to get these values handed back to us as the return value of the
        // validator method and we'll deal with the enum matching in rust-land
        let stdlib_validationresult = ctx.create_table()?;
        stdlib_validationresult
            .set("Ok", ctx.create_function(|_, ()| Ok(ValidationResult::Ok))?)?;
        stdlib_validationresult.set(
            "OkWithWarnings",
            ctx.create_function(|_, warnings: Variadic<String>| {
                Ok(ValidationResult::OkWithWarnings(warnings.to_vec()))
            })?,
        )?;
        stdlib_validationresult.set(
            "Error",
            ctx.create_function(|_, err_msg: String| Ok(ValidationResult::Error(err_msg)))?,
        )?;
        globals.set("ValidationResult", stdlib_validationresult)?;

        Ok(())
    })
}

#[test]
fn test_seatrial_stdlib_validationresult_ok() -> LuaResult<()> {
    let lua = Lua::default();
    attach_validationresult(&lua)?;

    lua.context(|ctx| {
        Ok(assert_eq!(
            ctx.load("ValidationResult.Ok()")
                .eval::<ValidationResult>()?,
            ValidationResult::Ok,
        ))
    })
}

#[test]
fn test_seatrial_stdlib_validationresult_ok_with_warnings() -> LuaResult<()> {
    let lua = Lua::default();
    attach_validationresult(&lua)?;

    lua.context(|ctx| {
        Ok(assert_eq!(
            ctx.load("ValidationResult.OkWithWarnings(\"yo, this is a test!\")")
                .eval::<ValidationResult>()?,
            ValidationResult::OkWithWarnings(vec!["yo, this is a test!".into()]),
        ))
    })
}

#[test]
fn test_seatrial_stdlib_validationresult_ok_with_warnings_multi() -> LuaResult<()> {
    let lua = Lua::default();
    attach_validationresult(&lua)?;

    lua.context(|ctx| {
        Ok(assert_eq!(
            ctx.load(
                "ValidationResult.OkWithWarnings(\"yo, this is a test!\", \"this is also a test\")"
            )
            .eval::<ValidationResult>()?,
            ValidationResult::OkWithWarnings(vec![
                "yo, this is a test!".into(),
                "this is also a test".into()
            ]),
        ))
    })
}

#[test]
fn test_seatrial_stdlib_validationresult_ok_with_warnings_req_argument() -> LuaResult<()> {
    let lua = Lua::default();
    attach_validationresult(&lua)?;

    lua.context(|ctx| {
        match ctx
            .load("ValidationResult.OkWithWarnings()")
            .eval::<ValidationResult>()
        {
            Ok(result) => panic!("expected to get a RuntimeError, got {:?}", result),
            Err(LuaError::RuntimeError(msg)) => {
                assert_eq!(msg, VALIDATION_RESULT_MISSING_WARNING_ARG_MSG);
                Ok(())
            }
            Err(error) => panic!("expected to get a RuntimeError, got {:?}", error),
        }
    })
}
