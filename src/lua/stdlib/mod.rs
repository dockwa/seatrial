use rlua::{Lua, Result as LuaResult};

pub mod validation_result;

pub use validation_result::{attach_validationresult, ValidationResult};

pub fn attach_seatrial_stdlib(lua: &Lua) -> LuaResult<()> {
    attach_validationresult(lua)?;
    Ok(())
}
