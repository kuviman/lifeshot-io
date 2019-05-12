use crate::*;

pub struct Connection<T> {
    ws: stdweb::web::WebSocket,
    phantom_data: PhantomData<T>,
}

impl<T: Message> Sender<T> for Connection<T> {
    fn send(&mut self, message: T) {
        self.ws
            .send_bytes(&serialize_message(message))
            .expect("Failed to send message");
    }
}

impl<T> Drop for Connection<T> {
    fn drop(&mut self) {
        self.ws.close();
    }
}

pub fn connect<S: Message, C: Message>(
    host: &str,
    port: u16,
    mut receiver: impl Receiver<S> + Send + 'static,
) -> impl Promise<Output = Connection<C>> {
    let ws = stdweb::web::WebSocket::new(&format!("ws://{}:{}", host, port)).unwrap();
    let (promise, promise_handle) = promise::channel();
    let connection = Connection {
        ws: ws.clone(),
        phantom_data: PhantomData,
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
        receiver.handle(deserialize_message(&data));
    });
    promise
}
