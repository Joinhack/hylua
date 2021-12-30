use crate::MakeLuaSvr;
use hyper::{self, Body, Request, Response, Server};
use mlua::Lua;
use std::fs::File;
use std::future::Future;
use std::io::Read;
use std::io::{Error, Result};
use std::net::SocketAddr;
use std::path::Path;
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

pub struct HttpServer {
    lua_file: String,
}

impl HttpServer {
    pub fn new(lua_file: String) -> HttpServer {
        HttpServer { lua_file }
    }

    async fn do_request(self: Rc<HttpServer>, req: Request<Body>) -> Result<Response<Body>> {
        Ok(Response::new("ok".into()))
    }

    pub async fn run(self: Rc<HttpServer>) {
        let lua = Rc::new(Lua::new());
        let path = Path::new(&self.lua_file);
        if !path.exists() {
            panic!("Lua file is not exists")
        }
        let mut file = File::open(path).unwrap();
        let mut source = Vec::<u8>::new();
        let _ = file.read_to_end(&mut source).unwrap();
        lua.load(&source).exec().unwrap();
        let make_service = MakeLuaSvr::new(lua);
        let addr = "0.0.0.0:8080";
        let addr: SocketAddr = addr.parse().unwrap();
        let server = Server::bind(&addr).executor(LocalExec).serve(make_service);
        let local = tokio::task::LocalSet::new();
        local.run_until(server).await.expect("cannot run server")
    }
}
