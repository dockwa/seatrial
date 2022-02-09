use rlua::{Lua, RegistryKey};

use std::collections::HashMap;
use std::io::{Error as IOError, Result as IOResult};

use crate::pipe_contents::PipeContents;

type HttpResponseTablePair = (&'static str, RegistryKey);

#[derive(Clone, Debug)]
pub struct HttpResponseTable {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub content_type: String,
    pub body: Vec<u8>,
    pub body_string: Option<String>,
}

impl HttpResponseTable {
    pub fn bind(self, lua: &Lua) -> BoundHttpResponseTable {
        BoundHttpResponseTable { lua, table: self }
    }
}

impl TryFrom<&PipeContents> for HttpResponseTable {
    type Error = IOError;

    fn try_from(it: &PipeContents) -> IOResult<Self> {
        match it {
            PipeContents::HttpResponse {
                body,
                content_type,
                headers,
                status_code,
            } => Ok(Self {
                body: body.clone(),
                body_string: String::from_utf8(body.clone()).ok(),
                content_type: content_type.clone(),
                headers: headers.clone(),
                status_code: *status_code,
            }),
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct BoundHttpResponseTable<'lua> {
    lua: &'lua Lua,
    table: HttpResponseTable,
}

impl<'a> BoundHttpResponseTable<'a> {
    fn iter(&'a self) -> BoundHttpResponseTableIter<'a> {
        BoundHttpResponseTableIter::create(self)
    }
}

impl<'a> IntoIterator for &'a BoundHttpResponseTable<'a> {
    type Item = HttpResponseTablePair;
    type IntoIter = BoundHttpResponseTableIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct BoundHttpResponseTableIter<'a> {
    child: &'a BoundHttpResponseTable<'a>,
    iter_state: usize,
}

impl<'a> BoundHttpResponseTableIter<'a> {
    pub fn create(child: &'a BoundHttpResponseTable<'a>) -> Self {
        Self {
            child,
            iter_state: 0,
        }
    }
}

impl<'a> Iterator for BoundHttpResponseTableIter<'a> {
    type Item = HttpResponseTablePair;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter_state += 1;
        match self.iter_state {
            1 => Some((
                "status_code",
                self.child.lua.context(|ctx| {
                    ctx.create_registry_value(self.child.table.status_code)
                        .expect("should have created status_code integer in registry")
                }),
            )),
            2 => Some((
                "headers",
                self.child.lua.context(|ctx| {
                    ctx.create_registry_value(self.child.table.headers.clone())
                        .expect("should have created headers table in registry")
                }),
            )),
            3 => Some((
                "content_type",
                self.child.lua.context(|ctx| {
                    ctx.create_registry_value(self.child.table.content_type.clone())
                        .expect("should have created content_type string in registry")
                }),
            )),
            4 => Some((
                "body",
                self.child.lua.context(|ctx| {
                    ctx.create_registry_value(self.child.table.body.clone())
                        .expect("should have created body table in registry")
                }),
            )),
            5 => Some((
                "body_string",
                self.child.lua.context(|ctx| {
                    ctx.create_registry_value(self.child.table.body_string.clone())
                        .expect("should have created body_string nilable-string in registry")
                }),
            )),
            _ => None,
        }
    }
}
