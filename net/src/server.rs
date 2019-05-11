use crate::*;

pub trait App: Send + 'static {
    type Client: Receiver<Self::ClientMessage>;
    type ServerMessage: Message;
    type ClientMessage: Message;
    fn connect(&mut self, sender: Box<Sender<Self::ServerMessage>>) -> Self::Client;
    const TICKS_PER_SECOND: f64;
    fn tick(&mut self);
}

struct Handler<T, P> {
    client: T,
    phantom_data: PhantomData<P>,
}

impl<M: Message, T: Receiver<M>> ws::Handler for Handler<T, M> {
    fn on_message(&mut self, message: ws::Message) -> ws::Result<()> {
        self.client
            .handle(deserialize_message(&message.into_data()));
        Ok(())
    }
}

struct Factory<T> {
    app: T,
}

impl<T> Factory<T> {
    fn new(app: T) -> Self {
        Self { app }
    }
}

impl<T: App> ws::Factory for Factory<Arc<Mutex<T>>> {
    type Handler = Handler<T::Client, T::ClientMessage>;

    fn connection_made(&mut self, sender: ws::Sender) -> Handler<T::Client, T::ClientMessage> {
        info!("New connection");
        let mut app = self.app.lock().unwrap();
        let client = app.connect(Box::new(sender));
        Handler {
            client,
            phantom_data: PhantomData,
        }
    }
}

pub struct Server<T: App> {
    ws: ws::WebSocket<Factory<Arc<Mutex<T>>>>,
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
        let app = Arc::new(Mutex::new(app));
        let factory = Factory::new(app.clone());
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
