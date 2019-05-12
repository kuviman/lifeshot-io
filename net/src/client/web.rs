use crate::*;

pub struct Connection<T> {
    phantom_data: PhantomData<T>,
}

impl<T: Message> Sender<T> for Connection<T> {
    fn send(&mut self, message: T) {}
}

impl<T> Drop for Connection<T> {
    fn drop(&mut self) {}
}

pub fn connect<S: Message, C: Message>(
    host: &str,
    port: u16,
    receiver: impl Receiver<S> + Send + 'static,
) -> impl Promise<Output = Connection<C>> {
    promise::ready(unimplemented!())
}
