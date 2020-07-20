#![allow(dead_code, unused_variables, unused_imports)]

use geng::net;
use geng::prelude::*;
use log::{debug, error, info, trace, warn};

mod client;
mod common_model;
#[cfg(not(target_arch = "wasm32"))]
mod server;

use client::*;
use common_model::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use server::*;

mod events;
use events::*;

#[derive(StructOpt, Debug, Clone)]
pub struct OptsNetOpts {
    #[structopt(long = "host")]
    host: Option<String>,
    #[structopt(long = "port")]
    port: Option<u16>,
    #[structopt(long = "addr")]
    addr: Option<String>,
}

impl OptsNetOpts {
    fn get(&self) -> NetOpts {
        NetOpts {
            host: self
                .host
                .as_deref()
                .or(option_env!("LIFESHOT_HOST"))
                .unwrap_or("127.0.0.1")
                .to_owned(),
            port: self
                .port
                .or(option_env!("LIFESHOT_PORT")
                    .map(|port| port.parse().expect("Failed to parse port")))
                .unwrap_or(1154),
            addr: self
                .addr
                .as_deref()
                .or(option_env!("LIFESHOT_ADDR"))
                .unwrap_or("ws://127.0.0.1:1154")
                .to_owned(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NetOpts {
    host: String,
    port: u16,
    addr: String,
}

#[derive(StructOpt, Debug, Clone)]
pub enum Command {
    #[structopt(name = "server-only")]
    ServerOnly,
    #[structopt(name = "with-server")]
    WithServer,
}

#[derive(StructOpt, Debug, Clone)]
pub struct Opts {
    #[structopt(long = "log-level")]
    log_level: Option<log::LevelFilter>,
    #[structopt(flatten)]
    net_opts: OptsNetOpts,
    #[structopt(long = "name", default_value = "<noname>")]
    name: String,
    #[structopt(subcommand)]
    command: Option<Command>,
}

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Ok(path) = std::env::var("CARGO_MANIFEST_DIR") {
            std::env::set_current_dir(std::path::Path::new(&path).join("static")).unwrap();
        } else {
            std::env::set_current_dir(std::env::current_exe().unwrap().parent().unwrap()).unwrap();
        }
    }
    logger::init();
    let opts: Opts = program_args::parse();
    info!("Options used:\n{:#?}", opts);
    let net_opts = opts.net_opts.get();
    if let Some(level) = opts.log_level {
        log::set_max_level(level);
    }
    info!("Net opts:\n{:#?}", net_opts);
    trace!("Initializing");

    #[cfg(target_arch = "wasm32")]
    let server = None::<()>;
    #[cfg(not(target_arch = "wasm32"))]
    let (server, server_handle) = if opts.command.is_some() {
        let server = Server::new(&net_opts);
        let server_handle = server.handle();
        ctrlc::set_handler({
            let server_handle = server_handle.clone();
            move || {
                server_handle.shutdown();
            }
        })
        .unwrap();
        (Some(server), Some(server_handle))
    } else {
        (None, None)
    };
    let client = match opts.command {
        Some(Command::ServerOnly) => false,
        _ => true,
    };

    #[cfg(not(target_arch = "wasm32"))]
    let server_thread = if let Some(server) = server {
        if client {
            Some(std::thread::spawn(move || server.run()))
        } else {
            server.run();
            None
        }
    } else {
        None
    };

    if client {
        ClientApp::run(&opts, &net_opts);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(server_thread) = server_thread {
            if client {
                server_handle.unwrap().shutdown();
            }
            server_thread.join().unwrap();
        }
    }
}
