use crate::*;

pub struct Connection<S: Message, C: Message> {
    sender: ws::Sender,
    broadcaster: ws::Sender,
    recv: std::sync::mpsc::Receiver<S>,
    thread_handle: Option<std::thread::JoinHandle<()>>,
    phantom_data: PhantomData<(S, C)>,
}

impl<S: Message, C: Message> Connection<S, C> {
    pub fn try_recv(&mut self) -> Option<S> {
        match self.recv.try_recv() {
            Ok(message) => Some(message),
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => panic!("Disconnected from server"),
        }
    }
    pub fn send(&mut self, message: C) {
        trace!("Sending message to server: {:?}", message);
        self.sender
            .send(ws::Message::Binary(serialize_message(message)))
            .expect("Failed to send message");
    }
}

impl<S: Message, C: Message> Drop for Connection<S, C> {
    fn drop(&mut self) {
        self.broadcaster.shutdown().unwrap();
        self.thread_handle.take().unwrap().join().unwrap();
    }
}

struct Handler<T: Message> {
    promise_handle: Option<promise::ChannelHandle<ws::Sender>>,
    recv_sender: std::sync::mpsc::Sender<T>,
    sender: ws::Sender,
}

impl<T: Message> ws::Handler for Handler<T> {
    fn on_open(&mut self, _: ws::Handshake) -> ws::Result<()> {
        info!("Connected to the server");
        self.promise_handle
            .take()
            .unwrap()
            .ready(self.sender.clone());
        Ok(())
    }
    fn on_message(&mut self, message: ws::Message) -> ws::Result<()> {
        let message = deserialize_message(&message.into_data());
        trace!("Got message from server: {:?}", message);
        self.recv_sender.send(message).unwrap();
        Ok(())
    }
}

struct Factory<T: Message> {
    promise_handle: Option<promise::ChannelHandle<ws::Sender>>,
    recv_sender: Option<std::sync::mpsc::Sender<T>>,
}

impl<T: Message> ws::Factory for Factory<T> {
    type Handler = Handler<T>;
    fn connection_made(&mut self, sender: ws::Sender) -> Handler<T> {
        Handler {
            promise_handle: self.promise_handle.take(),
            recv_sender: self.recv_sender.take().unwrap(),
            sender,
        }
    }
}

pub fn connect<S: Message, C: Message>(
    host: &str,
    port: u16,
) -> impl Promise<Output = Connection<S, C>> {
    let (promise, promise_handle) = promise::channel();
    let (recv_sender, recv) = std::sync::mpsc::channel();
    let factory = Factory {
        promise_handle: Some(promise_handle),
        recv_sender: Some(recv_sender),
    };
    let mut ws = ws::WebSocket::new(factory).unwrap();
    let mut broadcaster = Some(ws.broadcaster());
    ws.connect(url::Url::parse(&format!("ws://{}:{}", host, port)).unwrap())
        .unwrap();
    let mut thread_handle = Some(std::thread::spawn(move || {
        ws.run().unwrap();
    }));
    let mut recv = Some(recv);
    promise.map(move |sender| Connection {
        sender,
        broadcaster: broadcaster.take().unwrap(),
        recv: recv.take().unwrap(),
        thread_handle: thread_handle.take(),
        phantom_data: PhantomData,
    })
}
