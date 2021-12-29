use std::rc::Rc;
use tokio::{self, runtime};

use ding_push::*;

fn main() {
    let rt = runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();
    rt.block_on(async {
        let svr = Rc::new(http::HttpServer::new());
        svr.run().await;
    });
}
