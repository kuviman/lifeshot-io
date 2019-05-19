use crate::*;

pub trait App: Send + 'static {
    type Client: Receiver<Self::ClientMessage>;
    type ServerMessage: Message;
    type ClientMessage: Message;
    fn connect(&mut self, sender: Box<Sender<Self::ServerMessage>>) -> Self::Client;
    const TICKS_PER_SECOND: f64;
    fn tick(&mut self);
}

struct Handler<T: App> {
    app: Arc<Mutex<T>>,
    sender: ws::Sender,
    client: Option<T::Client>,
}

impl<T: App> ws::Handler for Handler<T> {
    fn on_open(&mut self, _: ws::Handshake) -> ws::Result<()> {
        self.client = Some(
            self.app
                .lock()
                .unwrap()
                .connect(Box::new(self.sender.clone())),
        );
        Ok(())
    }
    fn on_message(&mut self, message: ws::Message) -> ws::Result<()> {
        self.client
            .as_mut()
            .expect("Received a message before handshake")
            .handle(deserialize_message(&message.into_data()));
        Ok(())
    }
}

struct Factory<T: App> {
    app: Arc<Mutex<T>>,
}

impl<T: App> Factory<T> {
    fn new(app: T) -> Self {
        Self {
            app: Arc::new(Mutex::new(app)),
        }
    }
}

impl<T: App> ws::Factory for Factory<T> {
    type Handler = Handler<T>;

    fn connection_made(&mut self, sender: ws::Sender) -> Handler<T> {
        info!("New connection");
        Handler {
            app: self.app.clone(),
            sender,
            client: None,
        }
    }
}

pub struct Server<T: App> {
    ws: ws::WebSocket<Factory<T>>,
    app: Arc<Mutex<T>>,
}

#[derive(Clone)]
pub struct ServerHandle {
    sender: ws::Sender,
}

impl ServerHandle {
    pub fn shutdown(&self) {
        self.sender.shutdown().expect("Failed to shutdown server");
    }
}

impl<T: App> Server<T> {
    pub fn new(app: T, addr: impl std::net::ToSocketAddrs + Debug + Copy) -> Self {
        let factory = Factory::new(app);
        let app = factory.app.clone();
        let ws = ws::WebSocket::new(factory).unwrap();
        let ws = match ws.bind(addr) {
            Ok(ws) => ws,
            Err(e) => {
                error!("Failed to bind server to {:?}: {}", addr, e);
                panic!("{:?}", e);
            }
        };
        Self { ws, app }
    }
    pub fn handle(&self) -> ServerHandle {
        ServerHandle {
            sender: self.ws.broadcaster(),
        }
    }
    pub fn run(self) {
        info!("Starting the server");
        let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let server_thread = std::thread::spawn({
            let app = self.app;
            let running = running.clone();
            move || {
                while running.load(std::sync::atomic::Ordering::Relaxed) {
                    // TODO: smoother TPS
                    std::thread::sleep(std::time::Duration::from_millis(
                        (1000.0 / T::TICKS_PER_SECOND) as u64,
                    ));
                    let mut app = app.lock().unwrap();
                    app.tick();
                }
            }
        });
        match self.ws.run() {
            Ok(_) => {
                info!("Server finished successfully");
            }
            Err(e) => {
                error!("Server shutdown with error: {}", e);
                panic!("{:?}", e);
            }
        }
        running.store(false, std::sync::atomic::Ordering::Relaxed);
        server_thread.join().expect("Failed to join server thread");
    }
}
