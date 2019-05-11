use geng::prelude::*;
use log::{debug, error, info, trace, warn};

pub mod client;
pub mod server;

pub use server::{Server, ServerHandle};

pub trait Message: Serialize + for<'de> Deserialize<'de> + Send + 'static {}

fn serialize_message<T: Message>(message: T) -> Vec<u8> {
    bincode::serialize(&message).unwrap()
}

fn deserialize_message<T: Message>(data: &[u8]) -> T {
    bincode::deserialize(data).expect("Failed to deserialize message")
}

pub trait Sender<T>: Send {
    fn send(&mut self, message: T);
}

impl<T: Message> Sender<T> for ws::Sender {
    fn send(&mut self, message: T) {
        self.deref()
            .send(ws::Message::Binary(serialize_message(message)))
            .expect("Failed to send message");
    }
}

pub trait Receiver<T> {
    fn handle(&mut self, message: T);
}
