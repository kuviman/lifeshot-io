use super::*;

pub struct Events<T: Clone> {
    senders: Arc<Mutex<Vec<std::sync::mpsc::Sender<T>>>>,
}

impl<T: Clone> Events<T> {
    pub fn new() -> Self {
        Self {
            senders: Arc::new(Mutex::new(Vec::new())),
        }
    }
    pub fn subscribe(&self) -> std::sync::mpsc::Receiver<T> {
        let (sender, receiver) = std::sync::mpsc::channel();
        self.senders.lock().unwrap().push(sender);
        receiver
    }
    pub fn fire(&mut self, event: T) {
        let mut senders = self.senders.lock().unwrap();
        senders.retain(|sender| sender.send(event.clone()).is_ok());
    }
}
