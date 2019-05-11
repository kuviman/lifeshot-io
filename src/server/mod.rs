use crate::*;

pub struct Client {
    player_id: PlayerId,
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
        let mut model = self.model.lock().unwrap();
        model
            .players
            .get_mut(&self.player_id)
            .expect("Player not found")
            .action = message.action;
        self.sender.send(ServerMessage {
            model: model.clone(),
        });
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
    const TICKS_PER_SECOND: f64 = 20.0;
    fn connect(&mut self, mut sender: Box<net::Sender<ServerMessage>>) -> Client {
        let player = Player::new();
        let player_id = PlayerId::new();
        {
            let mut model = self.model.lock().unwrap();
            model.players.insert(player_id, player);
            sender.send(ServerMessage {
                model: model.clone(),
            });
        }
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
