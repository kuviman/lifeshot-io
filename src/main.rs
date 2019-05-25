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

#[derive(structopt::StructOpt, Debug, Clone)]
pub struct NetOpts {
    #[structopt(long = "host", default_value = "server.lifeshot.io")]
    host: String,
    #[structopt(long = "port", default_value = "1154")]
    port: u16,
    #[structopt(long = "extra-delay")]
    extra_delay: Option<u64>,
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
    #[structopt(long = "log-level")]
    log_level: Option<log::LevelFilter>,
    #[structopt(flatten)]
    net_opts: NetOpts,
    #[structopt(subcommand)]
    command: Option<Command>,
}

fn main() {
    logger::init();
    trace!("Initializing");

    #[cfg(target_arch = "wasm32")]
    let opts: Opts = structopt::StructOpt::from_iter({
        let mut args = Vec::<String>::new();
        args.push("lifeshot-io".to_owned()); // `Program` itself is the first arg
        let url = stdweb::web::window()
            .location()
            .expect("Failed to get window.location.href")
            .href()
            .expect("Failed to get window.location.href");
        let url = url::Url::parse(&url).expect("Failed to parse window.location.href");
        for (key, value) in url.query_pairs() {
            let key: &str = &key;
            let value: &str = &value;
            args.push("--".to_owned() + key);
            args.push(value.to_owned());
        }
        trace!("href => args: {:?}", args);
        args
    });
    #[cfg(not(target_arch = "wasm32"))]
    let opts: Opts = structopt::StructOpt::from_args();
    if let Some(level) = opts.log_level {
        log::set_max_level(level);
    }
    trace!("Options used:\n{:#?}", opts);

    #[cfg(target_arch = "wasm32")]
    let server = None::<()>;
    #[cfg(not(target_arch = "wasm32"))]
    let (server, server_handle) = if opts.command.is_some() {
        let server = Server::new(&opts.net_opts);
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
        let geng = Rc::new(Geng::new(geng::ContextOptions {
            title: "LifeShot.io".to_owned(),
            ..default()
        }));
        let app = ClientApp::new(&geng, opts.net_opts.clone());
        geng::run(geng, app);
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
