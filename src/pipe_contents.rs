use rlua::{Lua, RegistryKey, Value as LuaValue};

use std::collections::HashMap;
use std::io::{Error as IOError, Read, Result as IOResult};
use std::rc::Rc;

use crate::http_response_table::HttpResponseTable;
use crate::step_error::StepError;

#[derive(Debug)]
pub enum PipeContents {
    HttpResponse {
        body: Vec<u8>,
        content_type: String,
        headers: HashMap<String, String>,
        status_code: u16,
    },
    LuaReference(Rc<RegistryKey>),
}

impl PipeContents {
    pub fn to_lua(&self, lua: &Lua) -> Result<Option<Rc<RegistryKey>>, StepError> {
        match self {
            PipeContents::LuaReference(lref) => Ok(Some(lref.clone())),
            res @ PipeContents::HttpResponse { .. } => lua.context(|ctx| {
                let arg_table = ctx.create_table()?;
                for (key, val_rkey) in &HttpResponseTable::try_from(res)?.bind(lua) {
                    arg_table.set(key, ctx.registry_value::<LuaValue>(&val_rkey)?)?;
                }
                let registry_key = ctx.create_registry_value(arg_table)?;
                Ok(Some(Rc::new(registry_key)))
            }),
        }
    }
}

impl TryFrom<ureq::Response> for PipeContents {
    type Error = IOError;

    fn try_from(res: ureq::Response) -> IOResult<Self> {
        Ok(Self::HttpResponse {
            content_type: res.content_type().into(),
            status_code: res.status(),
            headers: {
                let headers_names = res.headers_names();
                let mut headers = HashMap::with_capacity(headers_names.len());

                for header_name in headers_names {
                    if let Some(header_val) = res.header(&header_name) {
                        headers.insert(header_name, header_val.into());
                    }
                }

                headers
            },
            body: {
                let len: Option<usize> =
                    res.header("Content-Length").and_then(|cl| cl.parse().ok());

                let mut body: Vec<u8> = if let Some(capacity) = len {
                    Vec::with_capacity(capacity)
                } else {
                    Vec::new()
                };

                res.into_reader().read_to_end(&mut body)?;

                body
            },
        })
    }
}
