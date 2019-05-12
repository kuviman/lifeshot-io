use geng::prelude::*;
use log::{debug, error, info, trace, warn};

#[cfg(target_arch = "wasm32")]
#[path = "client/web.rs"]
pub mod client;

#[cfg(not(target_arch = "wasm32"))]
#[path = "client/native.rs"]
pub mod client;

#[cfg(not(target_arch = "wasm32"))]
pub mod server;

#[cfg(not(target_arch = "wasm32"))]
pub use server::{Server, ServerHandle};

pub trait Message: Serialize + for<'de> Deserialize<'de> + Send + 'static {}

fn serialize_message<T: Message>(message: T) -> Vec<u8> {
    serde_json::to_vec(&message).unwrap()
}

fn deserialize_message<T: Message>(data: &[u8]) -> T {
    serde_json::from_slice(data).expect("Failed to deserialize message")
}

pub trait Sender<T>: Send {
    fn send(&mut self, message: T);
}

#[cfg(not(target_arch = "wasm32"))]
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
