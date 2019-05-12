use crate::*;

pub struct Connection<T> {
    sender: ws::Sender,
    broadcaster: ws::Sender,
    phantom_data: PhantomData<T>,
    thread_handle: Option<std::thread::JoinHandle<()>>,
}

impl<T: Message> Sender<T> for Connection<T> {
    fn send(&mut self, message: T) {
        self.sender
            .send(ws::Message::Binary(serialize_message(message)))
            .expect("Failed to send message");
    }
}

impl<T> Drop for Connection<T> {
    fn drop(&mut self) {
        self.broadcaster.shutdown().unwrap();
        self.thread_handle.take().unwrap().join().unwrap();
    }
}

struct Handler<T> {
    promise_handle: Option<promise::ChannelHandle<ws::Sender>>,
    sender: Option<ws::Sender>,
    receiver: Box<Receiver<T> + Send>,
}

impl<T: Message> ws::Handler for Handler<T> {
    fn on_open(&mut self, _: ws::Handshake) -> ws::Result<()> {
        info!("Connected to the server");
        self.promise_handle
            .take()
            .unwrap()
            .ready(self.sender.take().unwrap());
        Ok(())
    }
    fn on_message(&mut self, message: ws::Message) -> ws::Result<()> {
        let message = deserialize_message(&message.into_data());
        self.receiver.handle(message);
        Ok(())
    }
}

struct Factory<S> {
    promise_handle: Option<promise::ChannelHandle<ws::Sender>>,
    receiver: Option<Box<Receiver<S> + Send>>,
}

impl<S: Message> ws::Factory for Factory<S> {
    type Handler = Handler<S>;
    fn connection_made(&mut self, sender: ws::Sender) -> Handler<S> {
        Handler {
            promise_handle: self.promise_handle.take(),
            sender: Some(sender),
            receiver: self.receiver.take().unwrap(),
        }
    }
}

pub fn connect<S: Message, C: Message>(
    host: &str,
    port: u16,
    receiver: impl Receiver<S> + Send + 'static,
) -> impl Promise<Output = Connection<C>> {
    let (promise, promise_handle) = promise::channel();
    let factory = Factory {
        promise_handle: Some(promise_handle),
        receiver: Some(Box::new(receiver)),
    };
    let mut ws = ws::WebSocket::new(factory).unwrap();
    let mut broadcaster = Some(ws.broadcaster());
    ws.connect(url::Url::parse(&format!("ws://{}:{}", host, port)).unwrap())
        .unwrap();
    let mut thread_handle = Some(std::thread::spawn(move || {
        ws.run().unwrap();
    }));
    promise.map(move |sender| Connection {
        sender,
        broadcaster: broadcaster.take().unwrap(),
        phantom_data: PhantomData,
        thread_handle: thread_handle.take(),
    })
}
