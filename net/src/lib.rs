#[cfg(target_arch = "wasm32")]
#[macro_use]
extern crate stdweb;

use geng::prelude::*;
use log::{debug, error, info, trace, warn};

pub mod client;

#[cfg(not(target_arch = "wasm32"))]
pub mod server;

#[cfg(not(target_arch = "wasm32"))]
pub use server::{Server, ServerHandle};

pub trait Message: Debug + Serialize + for<'de> Deserialize<'de> + Send + 'static {}

impl<T: Debug + Serialize + for<'de> Deserialize<'de> + Send + 'static> Message for T {}

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
