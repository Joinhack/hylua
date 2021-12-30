use log::error;
use std::env;
use std::rc::Rc;
use tokio::{self, runtime};

use ding_push::*;

fn main() {
    let mut args = env::args();
    if args.len() < 2 {
        eprintln!("Usage {} luafile", args.nth(0).unwrap());
        return;
    }
    let lua_file = args.nth(1).unwrap();
    let rt = runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();
    rt.block_on(async move {
        let svr = Rc::new(http::HttpServer::new(lua_file));
        svr.run().await;
    });
}
