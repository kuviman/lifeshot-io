#![allow(dead_code, unused_variables, unused_imports)]

use geng::prelude::*;
use log::{debug, error, info, trace, warn};

mod client;
mod model;
#[cfg(not(target_arch = "wasm32"))]
mod server;

use client::*;
use model::*;
#[cfg(not(target_arch = "wasm32"))]
use server::*;

#[derive(structopt::StructOpt, Debug, Clone)]
pub struct NetOpts {
    #[structopt(long = "host", default_value = "server.lifeshot.io")]
    host: String,
    #[structopt(long = "port", default_value = "1154")]
    port: u16,
}

#[derive(structopt::StructOpt, Debug)]
enum Command {
    #[structopt(name = "server-only")]
    ServerOnly,
    #[structopt(name = "with-server")]
    WithServer,
}

#[derive(structopt::StructOpt, Debug)]
struct Opts {
    #[structopt(flatten)]
    net_opts: NetOpts,
    #[structopt(subcommand)]
    command: Option<Command>,
}

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init_from_env(env_logger::Env::new().filter_or("LISH_LOG", "lish,net"));
    trace!("Initializing");

    #[cfg(target_arch = "wasm32")]
    let opts: Opts = structopt::StructOpt::from_iter(std::iter::empty::<String>());
    #[cfg(not(target_arch = "wasm32"))]
    let opts: Opts = structopt::StructOpt::from_args();
    trace!("Options used:\n{:#?}", opts);

    #[cfg(target_arch = "wasm32")]
    let server = None::<()>;
    #[cfg(not(target_arch = "wasm32"))]
    let (server, server_handle) = if opts.command.is_some() {
        let server = net::Server::new(
            Server::new(),
            (opts.net_opts.host.as_str(), opts.net_opts.port),
        );
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
        let context = Rc::new(geng::Context::new(geng::ContextOptions {
            title: "Lish".to_owned(),
            ..default()
        }));
        let app = ClientApp::new(&context, opts.net_opts.clone());
        geng::run(context, app);
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
