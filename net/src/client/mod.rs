use crate::*;

#[cfg(target_arch = "wasm32")]
#[path = "web.rs"]
mod _impl;

#[cfg(not(target_arch = "wasm32"))]
#[path = "native.rs"]
mod _impl;

pub struct Connection<S: Message, C: Message> {
    inner: _impl::Connection<S, C>,
}

impl<S: Message, C: Message> Connection<S, C> {
    pub fn try_recv(&mut self) -> Option<S> {
        self.inner.try_recv()
    }
}

impl<S: Message, C: Message> Sender<C> for Connection<S, C> {
    fn send(&mut self, message: C) {
        self.inner.send(message);
    }
}

pub fn connect<S: Message, C: Message>(
    host: &str,
    port: u16,
) -> impl Promise<Output = Connection<S, C>> {
    _impl::connect(host, port).map(|inner| Connection { inner })
}