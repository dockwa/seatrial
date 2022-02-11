use rlua::{Context, Lua, RegistryKey, Result as LuaResult};

use std::path::PathBuf;
use std::rc::Rc;

use crate::pipe_contents::PipeContents;
use crate::step_error::StepError;

pub mod stdlib;
use stdlib::attach_seatrial_stdlib;

#[derive(Debug)]
pub struct LuaForPipeline {
    pub lua: Lua,

    user_script_registry_key: Rc<RegistryKey>,
}

impl LuaForPipeline {
    pub fn new(user_script_path: &PathBuf) -> LuaResult<Self> {
        let lua = Lua::default();
        attach_seatrial_stdlib(&lua)?;

        let user_script_registry_key = attach_user_script(&lua, user_script_path)?;

        Ok(Self {
            lua,
            user_script_registry_key,
        })
    }

    /// a convenience wrapper to delegate to the inner lua object
    pub fn context<F, R>(&self, callback: F) -> R
    where
        F: FnOnce(Context) -> R,
    {
        self.lua.context(callback)
    }

    // TODO: this returning StepError is a holdover from when this function wasn't part of
    // LuaForPipeline and there was more spaghetti in the codebase. in this new keto world we have
    // less spaghetti, and this needs refactored
    pub fn run_user_script_function(
        &self,
        name: &str,
        pipe_data: Option<&PipeContents>,
    ) -> Result<Rc<RegistryKey>, StepError> {
        self.context(|ctx| {
            let lua_func = ctx
                .registry_value::<rlua::Table>(&self.user_script_registry_key)?
                .get::<_, rlua::Function>(name)?;
            let script_arg = match pipe_data {
                Some(lval) => match lval.to_lua(&self.lua)? {
                    Some(rkey) => ctx.registry_value::<rlua::Value>(&rkey)?,
                    None => rlua::Nil,
                },
                None => rlua::Nil,
            };
            let result = lua_func.call::<rlua::Value, rlua::Value>(script_arg)?;
            let registry_key = ctx.create_registry_value(result)?;
            Ok(Rc::new(registry_key))
        })
    }
}

fn attach_user_script(lua: &Lua, user_script_path: &PathBuf) -> LuaResult<Rc<RegistryKey>> {
    let fpath = if let Some(parent) = user_script_path.parent() {
        let mut ret = parent.to_path_buf();
        ret.push("?.lua");
        ret.to_string_lossy().into_owned()
    } else {
        user_script_path.to_string_lossy().into_owned()
    };
    let fname = user_script_path
        .file_stem()
        .unwrap_or_else(|| user_script_path.as_os_str())
        .to_string_lossy();

    lua.context(|ctx| {
        ctx.load(&format!(
            "package.path = package.path .. \";{}\"; _user_script = require('{}')",
            fpath, fname
        ))
        .set_name(&format!("user_script<{}>", fpath))?
        .exec()?;

        let user_script = ctx.globals().get::<_, rlua::Table>("_user_script")?;

        Ok(Rc::new(ctx.create_registry_value(user_script)?))
    })
}
