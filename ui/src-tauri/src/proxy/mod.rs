mod server;
mod socks;
mod rewrite;

use std::sync::atomic::{AtomicBool, Ordering};

static CONNECTED: AtomicBool = AtomicBool::new(false);

pub fn set_proxy_connected(val: bool) {
    CONNECTED.store(val, Ordering::Relaxed);
}

pub use server::{start_local_proxy_server, LOCAL_PROXY_PORT};
