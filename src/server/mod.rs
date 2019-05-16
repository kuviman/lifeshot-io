use crate::*;

mod model;

use model::*;

pub struct Client {
    player_id: Id,
    model: Arc<Mutex<Model>>,
    sender: Box<net::Sender<ServerMessage>>,
}

impl Drop for Client {
    fn drop(&mut self) {
        // TODO: remove the player
    }
}

impl net::Receiver<ClientMessage> for Client {
    fn handle(&mut self, message: ClientMessage) {
        let reply = match message {
            ClientMessage::Action(_) => true,
            _ => false,
        };
        let mut model = self.model.lock().unwrap();
        model.handle(self.player_id, message);
        if reply {
            self.sender.send(ServerMessage {
                client_player_id: self.player_id,
                model: model.clone(),
            });
        }
    }
}

pub struct Server {
    model: Arc<Mutex<Model>>,
}

impl Server {
    pub fn new() -> Self {
        Self {
            model: Arc::new(Mutex::new(default())),
        }
    }
}

impl net::server::App for Server {
    type Client = Client;
    type ServerMessage = ServerMessage;
    type ClientMessage = ClientMessage;
    const TICKS_PER_SECOND: f64 = 60.0;
    fn connect(&mut self, mut sender: Box<net::Sender<ServerMessage>>) -> Client {
        if let Ok(cmd) = std::env::var("NEW_PLAYER_CMD") {
            std::process::Command::new(cmd)
                .spawn()
                .expect("Failed to run NEW_PLAYER_CMD");
        }
        let player_id = {
            let mut model = self.model.lock().unwrap();
            let player_id = model.new_player();
            sender.send(ServerMessage {
                client_player_id: player_id,
                model: model.clone(),
            });
            player_id
        };
        Client {
            model: self.model.clone(),
            player_id,
            sender,
        }
    }
    fn tick(&mut self) {
        self.model
            .lock()
            .unwrap()
            .update(1.0 / Self::TICKS_PER_SECOND as f32);
    }
}
