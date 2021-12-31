use hyper::server::conn::AddrStream;
use hyper::service::Service;
use hyper::{self, Body, Request, Response};
use log::error;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use mlua::{
    Error as LuaError, Function, Lua, String as LuaString, Table, UserData, UserDataMethods,
    Value as LuaValue,
};

pub(crate) struct LuaRequest {
    remote_addr: SocketAddr,
    inner: Request<Body>,
}

impl UserData for LuaRequest {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("remote_addr", |_lua, req, ()| {
            Ok(req.remote_addr.to_string())
        });
        methods.add_method("get_header", |_lua, req, name: String| {
            Ok(req
                .inner
                .headers()
                .get(name)
                .map(|val| val.to_str().unwrap_or_default().to_string())
                .unwrap_or_default())
        });
    }
}

pub struct LuaSvr {
    lua: Rc<Lua>,
    remote_addr: SocketAddr,
}

impl LuaSvr {
    pub fn new(lua: Rc<Lua>, remote_addr: SocketAddr) -> LuaSvr {
        LuaSvr { lua, remote_addr }
    }

    async fn do_request(lua: Rc<Lua>, lua_req: LuaRequest) -> Result<Response<Body>, LuaError> {
        let handle: Function = lua.globals().get("do_request")?;
        let lua_tab = handle.call_async::<_, Table>(lua_req).await?;
        let body: LuaString = lua_tab.get("body")?;
        let status: LuaValue = lua_tab.get("status")?;
        let status: u16 = match status {
            LuaValue::Nil => 200,
            LuaValue::Integer(s) => s as u16,
            _ => 200,
        };
        let resp = Response::builder()
            .status(status)
            .body(body.as_bytes().to_vec().into())
            .unwrap();
        Ok(resp)
    }
}

impl Service<Request<Body>> for LuaSvr {
    type Response = Response<Body>;

    type Error = LuaError;

    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let lua = self.lua.clone();
        let remote_addr = self.remote_addr;
        Box::pin(async move {
            let lua_req = LuaRequest {
                remote_addr,
                inner: req,
            };
            match Self::do_request(lua, lua_req).await {
                Ok(resp) => Ok(resp),
                Err(e) => {
                    error!("{}", e.to_string());
                    Ok(Response::builder()
                        .status(500)
                        .body(Body::from("Intenal Error"))
                        .unwrap_or_default())
                }
            }
        })
    }
}

pub struct MakeLuaSvr {
    lua: Rc<Lua>,
}

impl MakeLuaSvr {
    pub fn new(lua: Rc<Lua>) -> MakeLuaSvr {
        MakeLuaSvr { lua }
    }
}

impl Service<&AddrStream> for MakeLuaSvr {
    type Response = LuaSvr;

    type Error = hyper::Error;

    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, _ctx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, stream: &AddrStream) -> Self::Future {
        let lua = self.lua.clone();
        let remote_addr = stream.remote_addr();
        Box::pin(async move { Ok(LuaSvr { lua, remote_addr }) })
    }
}

#[cfg(test)]
mod tests {
    use crate::{LuaRequest, LuaSvr};
    use hyper::body::to_bytes;
    use hyper::{Body, Request};
    use mlua::{chunk, Lua};
    use std::rc::Rc;
    use tokio::{self, runtime};

    #[test]
    fn test_lua_svr() {
        let rt = runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .unwrap();
        rt.block_on(async {
            let req = Request::builder()
                .uri("https://www.rust-lang.org/")
                .header("Host", "my-awesome-agent/1.0")
                .body(Body::from(""))
                .unwrap();
            let lua = Lua::new();
            lua.load(chunk! {
                function do_request(req)
                    local h = req:get_header("Host")
                    return {
                        ["header"] = {
                            ["content-type"] = "text/plain;charset=UTF-8"
                        },
                        ["body"] = h,
                    }
                end
            })
            .exec()
            .unwrap();
            let lua = Rc::new(lua);
            let l_req = LuaRequest {
                remote_addr: "127.0.0.1:8080".parse().unwrap(),
                inner: req,
            };

            LuaSvr::new(lua.clone(), "127.0.0.1:8080".parse().unwrap());
            let resp = LuaSvr::do_request(lua, l_req).await.unwrap();
            assert_eq!(resp.status(), 200);
            let vec: Vec<u8> = to_bytes(resp.into_body()).await.unwrap().to_vec();
            assert_eq!(
                std::str::from_utf8(&vec[..]).unwrap(),
                "my-awesome-agent/1.0"
            );
        });
    }
}
