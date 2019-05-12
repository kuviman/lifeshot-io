use crate::*;

pub struct ClientApp {
    context: Rc<geng::Context>,
    action: Arc<Mutex<Action>>,
    model: Arc<Mutex<Option<Model>>>,
    connection: Arc<Mutex<Option<net::client::Connection<ClientMessage>>>>,
    connection_promise: Box<Promise<Output = net::client::Connection<ClientMessage>>>,
}

impl ClientApp {
    pub fn new(context: &Rc<geng::Context>, net_opts: NetOpts) -> Self {
        struct Receiver {
            model: Arc<Mutex<Option<Model>>>,
            action: Arc<Mutex<Action>>,
            connection: Arc<Mutex<Option<net::client::Connection<ClientMessage>>>>,
        }
        impl net::Receiver<ServerMessage> for Receiver {
            fn handle(&mut self, message: ServerMessage) {
                *self.model.lock().unwrap() = Some(message.model);
                use net::Sender;
                if let Some(connection) = self.connection.lock().unwrap().as_mut() {
                    connection.send(ClientMessage {
                        action: self.action.lock().unwrap().clone(),
                    });
                }
            }
        }
        let model = Arc::new(Mutex::new(None));
        let action = Arc::new(Mutex::new(default()));
        let connection = Arc::new(Mutex::new(None));
        let connection_promise = net::client::connect(
            &net_opts.host,
            net_opts.port,
            Receiver {
                action: action.clone(),
                model: model.clone(),
                connection: connection.clone(),
            },
        );
        Self {
            context: context.clone(),
            action,
            model,
            connection,
            connection_promise: Box::new(connection_promise),
        }
    }
}

impl geng::App for ClientApp {
    fn update(&mut self, delta_time: f64) {
        {
            let mut connection = self.connection.lock().unwrap();
            if connection.is_none() && self.connection_promise.ready() {
                *connection = Some(self.connection_promise.unwrap());
                use net::Sender;
                connection.as_mut().unwrap().send(ClientMessage {
                    action: self.action.lock().unwrap().clone(),
                });
            }
        }
        {
            let mut action = self.action.lock().unwrap();
            action.target_vel = vec2(0.0, 0.0);
            if self.context.window().is_key_pressed(geng::Key::W) {
                action.target_vel.y += 1.0;
            }
            if self.context.window().is_key_pressed(geng::Key::A) {
                action.target_vel.x -= 1.0;
            }
            if self.context.window().is_key_pressed(geng::Key::S) {
                action.target_vel.y -= 1.0;
            }
            if self.context.window().is_key_pressed(geng::Key::D) {
                action.target_vel.x += 1.0;
            }
        }
    }
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        ugli::clear(framebuffer, Some(Color::BLACK), None);
        let framebuffer_size = framebuffer.get_size().map(|x| x as f32);
        let center = framebuffer_size / 2.0;
        let scale = framebuffer_size.y / 10.0;
        if let Some(model) = self.model.lock().unwrap().as_ref() {
            for player in model.players.values() {
                self.context.draw_2d().ellipse(
                    framebuffer,
                    player.pos * scale + center,
                    vec2(1.0, 1.0) * scale * player.size,
                    Color::WHITE,
                );
            }
        };
    }
}
