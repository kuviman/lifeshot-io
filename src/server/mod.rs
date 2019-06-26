use crate::*;

mod model;

use model::*;

struct Client {
    player_id: Id,
    name: Option<String>,
    model: Arc<Mutex<Model>>,
    events: std::sync::mpsc::Receiver<common_model::Event>,
    sender: Box<net::Sender<ServerMessage>>,
}

impl Drop for Client {
    fn drop(&mut self) {
        // TODO: remove the player
        if let Some(name) = &self.name {
            info!("{:?} disconnected", name);
        }
    }
}

impl net::Receiver<ClientMessage> for Client {
    fn handle(&mut self, message: ClientMessage) {
        let reply = match &message {
            ClientMessage::Action(_) => true,
            ClientMessage::Spawn => false,
            ClientMessage::SetName(name) => {
                self.name = Some(name.clone());
                info!("{:?} joined the game", name);
                if let Ok(cmd) = std::env::var("NEW_PLAYER_CMD") {
                    std::process::Command::new(cmd)
                        .arg(name)
                        .spawn()
                        .expect("Failed to run NEW_PLAYER_CMD");
                }
                false
            }
        };
        let mut model = self.model.lock().unwrap();
        model.handle(self.player_id, message);
        if reply {
            self.sender.send(ServerMessage {
                client_player_id: self.player_id,
                model: model.to_message(),
                events: self.events.try_iter().collect(),
            });
        }
    }
}
struct ServerApp {
    model: Arc<Mutex<Model>>,
}
impl net::server::App for ServerApp {
    type Client = Client;
    type ServerMessage = ServerMessage;
    type ClientMessage = ClientMessage;
    fn connect(&mut self, mut sender: Box<net::Sender<ServerMessage>>) -> Client {
        let (player_id, events) = {
            let mut model = self.model.lock().unwrap();
            let player_id = model.new_player();
            sender.send(ServerMessage {
                client_player_id: player_id,
                model: model.to_message(),
                events: model.initial_events(),
            });
            (player_id, model.events.subscribe())
        };
        Client {
            model: self.model.clone(),
            name: None,
            player_id,
            sender,
            events,
        }
    }
}

pub struct Server {
    model: Arc<Mutex<Model>>,
    server: net::Server<ServerApp>,
}

impl Server {
    const TICKS_PER_SECOND: f64 = Model::TICKS_PER_SECOND;
    pub fn new(net_opts: &NetOpts) -> Self {
        let model = Arc::new(Mutex::new(default()));
        Self {
            model: model.clone(),
            server: net::Server::new(
                ServerApp {
                    model: model.clone(),
                },
                (net_opts.host.as_str(), net_opts.port),
            ),
        }
    }
    pub fn handle(&self) -> net::ServerHandle {
        self.server.handle()
    }
    pub fn run(self) {
        let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let server_thread = std::thread::spawn({
            let model = self.model;
            let running = running.clone();
            move || {
                while running.load(std::sync::atomic::Ordering::Relaxed) {
                    // TODO: smoother TPS
                    std::thread::sleep(std::time::Duration::from_millis(
                        (1000.0 / Self::TICKS_PER_SECOND) as u64,
                    ));
                    let mut model = model.lock().unwrap();
                    model.tick();
                }
            }
        });
        self.server.run();
        running.store(false, std::sync::atomic::Ordering::Relaxed);
        server_thread.join().expect("Failed to join server thread");
    }
}
