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
pub struct NetOpts {
    #[structopt(long = "host", default_value = "server.lifeshot.io")]
    host: String,
    #[structopt(long = "port", default_value = "1154")]
    port: u16,
    #[structopt(long = "addr", default_value = "wss://server.lifeshot.io")]
    addr: String,
    #[structopt(long = "extra-delay")]
    extra_delay: Option<u64>,
}

#[derive(StructOpt, Debug)]
enum Command {
    #[structopt(name = "server-only")]
    ServerOnly,
    #[structopt(name = "with-server")]
    WithServer,
}

#[derive(StructOpt, Debug)]
struct Opts {
    #[structopt(long = "log-level")]
    log_level: Option<log::LevelFilter>,
    #[structopt(flatten)]
    net_opts: NetOpts,
    #[structopt(long = "name", default_value = "<noname>")]
    name: String,
    #[structopt(subcommand)]
    command: Option<Command>,
}

fn main() {
    #[cfg(not(any(target_arch = "asmjs", target_arch = "wasm32")))]
    {
        if let Ok(path) = std::env::var("CARGO_MANIFEST_DIR") {
            std::env::set_current_dir(std::path::Path::new(&path).join("static")).unwrap();
        } else {
            std::env::set_current_dir(std::env::current_exe().unwrap().parent().unwrap()).unwrap();
        }
    }
    logger::init();
    let opts: Opts = program_args::parse();
    if let Some(level) = opts.log_level {
        log::set_max_level(level);
    }
    info!("Options used:\n{:#?}", opts);
    trace!("Initializing");

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
        let app = geng::LoadingScreen::new(
            &geng,
            geng::EmptyLoadingScreen,
            geng::LoadAsset::load(&geng, "."),
            {
                let geng = geng.clone();
                move |assets| {
                    ClientApp::new(
                        &geng,
                        opts.name.clone(),
                        opts.net_opts.clone(),
                        assets.unwrap(),
                    )
                }
            },
        );
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
