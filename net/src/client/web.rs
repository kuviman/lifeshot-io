use crate::*;

pub struct Connection<S: Message, C: Message> {
    ws: stdweb::web::WebSocket,
    recv: std::sync::mpsc::Receiver<S>,
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
}

impl<S: Message, C: Message> Sender<C> for Connection<S, C> {
    fn send(&mut self, message: C) {
        self.ws
            .send_bytes(&serialize_message(message))
            .expect("Failed to send message");
    }
}

impl<S: Message, C: Message> Drop for Connection<S, C> {
    fn drop(&mut self) {
        self.ws.close();
    }
}

pub fn connect<S: Message, C: Message>(
    host: &str,
    port: u16,
) -> impl Promise<Output = Connection<S, C>> {
    let ws = stdweb::web::WebSocket::new(&format!("ws://{}:{}", host, port)).unwrap();
    let (promise, promise_handle) = promise::channel();
    let (recv_sender, recv) = std::sync::mpsc::channel();
    let connection = Connection {
        ws: ws.clone(),
        phantom_data: PhantomData,
        recv,
    };
    let mut promise_handle = Some(promise_handle);
    let mut connection = Some(connection);
    use stdweb::web::IEventTarget;
    ws.add_event_listener(move |event: stdweb::web::event::SocketOpenEvent| {
        promise_handle
            .take()
            .unwrap()
            .ready(connection.take().unwrap());
    });
    ws.set_binary_type(stdweb::web::SocketBinaryType::ArrayBuffer);
    ws.add_event_listener(move |event: stdweb::web::event::SocketMessageEvent| {
        use stdweb::web::event::IMessageEvent;
        let data: Vec<u8> = event.data().into_array_buffer().unwrap().into();
        let message = deserialize_message(&data);
        recv_sender.send(message).unwrap();
    });
    promise
}
