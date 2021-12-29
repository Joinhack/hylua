use crate::MakeLuaSvr;
use hyper::{self, Body, Request, Response, Server};
use mlua::Lua;
use std::future::Future;
use std::io::{Error, Result};
use std::net::SocketAddr;
use std::rc::Rc;
use tokio::{self, runtime};

#[derive(Clone, Copy)]
struct LocalExec;

impl<F> hyper::rt::Executor<F> for LocalExec
where
    F: std::future::Future + 'static,
{
    fn execute(&self, fut: F) {
        tokio::task::spawn_local(fut);
    }
}

pub struct HttpServer {}

impl HttpServer {
    pub fn new() -> HttpServer {
        HttpServer {}
    }

    async fn do_request(self: Rc<HttpServer>, req: Request<Body>) -> Result<Response<Body>> {
        Ok(Response::new("ok".into()))
    }

    pub async fn run(self: Rc<HttpServer>) {
        let lua = Rc::new(Lua::new());
        let make_service = MakeLuaSvr::new(lua);
        let addr = "0.0.0.0:8080";
        let addr: SocketAddr = addr.parse().unwrap();
        let server = Server::bind(&addr).executor(LocalExec).serve(make_service);
        let local = tokio::task::LocalSet::new();
        local.run_until(server).await.expect("cannot run server")
    }
}
