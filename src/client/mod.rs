use crate::*;

mod circle_renderer;
mod model;

use circle_renderer::CircleRenderer;
use model::*;

pub struct ClientApp {
    context: Rc<geng::Context>,
    circle_renderer: CircleRenderer,
    action: Arc<Mutex<Action>>,
    recv: Arc<Mutex<Option<ServerMessage>>>,
    model: Model,
    mouse_pos: Vec2<f32>,
    connection: Arc<Mutex<Option<net::client::Connection<ClientMessage>>>>,
    connection_promise: Box<Promise<Output = net::client::Connection<ClientMessage>>>,
}

impl ClientApp {
    const CAMERA_FOV: f32 = 30.0;

    pub fn new(context: &Rc<geng::Context>, net_opts: NetOpts) -> Self {
        struct Receiver {
            recv: Arc<Mutex<Option<ServerMessage>>>,
            net_delay: Option<u64>,
            action: Arc<Mutex<Action>>,
            connection: Arc<Mutex<Option<net::client::Connection<ClientMessage>>>>,
            timer: Timer,
            max_ping: f64,
        }
        impl net::Receiver<ServerMessage> for Receiver {
            fn handle(&mut self, message: ServerMessage) {
                if let Some(delay) = self.net_delay {
                    std::thread::sleep(std::time::Duration::from_millis(delay));
                }
                *self.recv.lock().unwrap() = Some(message);
                let p = self.timer.tick();
                if p > self.max_ping {
                    self.max_ping = p;
                    debug!("New max ping: {} ms", (p * 1000.0) as u64);
                }
                use net::Sender;
                if let Some(connection) = self.connection.lock().unwrap().as_mut() {
                    connection.send(ClientMessage {
                        action: self.action.lock().unwrap().clone(),
                    });
                }
            }
        }
        let recv = Arc::new(Mutex::new(None));
        let action = Arc::new(Mutex::new(default()));
        let connection = Arc::new(Mutex::new(None));
        let connection_promise = net::client::connect(
            &net_opts.host,
            net_opts.port,
            Receiver {
                net_delay: net_opts.extra_delay,
                action: action.clone(),
                recv: recv.clone(),
                connection: connection.clone(),
                timer: Timer::new(),
                max_ping: 0.0,
            },
        );
        Self {
            context: context.clone(),
            circle_renderer: CircleRenderer::new(context),
            action,
            recv,
            model: Model::new(),
            connection,
            mouse_pos: vec2(0.0, 0.0),
            connection_promise: Box::new(connection_promise),
        }
    }
}

impl geng::App for ClientApp {
    fn update(&mut self, delta_time: f64) {
        {
            let mut recv = self.recv.lock().unwrap();
            if let Some(message) = recv.take() {
                self.model.recv(message);
            }
        }
        self.model.update(delta_time as f32);
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
            action.shoot = self
                .context
                .window()
                .is_button_pressed(geng::MouseButton::Left);
            action.aim = self.mouse_pos;
        }
    }
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        let rules = &self.model.rules;

        ugli::clear(framebuffer, Some(Color::BLACK), None);
        let framebuffer_size = framebuffer.get_size().map(|x| x as f32);
        let center = framebuffer_size / 2.0;
        let scale = framebuffer_size.y / Self::CAMERA_FOV;

        let view_matrix = Mat4::scale(vec3(framebuffer_size.y / framebuffer_size.x, 1.0, 1.0))
            * Mat4::scale_uniform(2.0 / Self::CAMERA_FOV);
        // * Mat4::translate(-self.camera_pos.extend(0.0));
        self.mouse_pos = {
            let mouse_pos = self.context.window().mouse_pos().map(|x| x as f32);
            let mouse_pos = vec2(
                mouse_pos.x / framebuffer_size.x * 2.0 - 1.0,
                mouse_pos.y / framebuffer_size.y * 2.0 - 1.0,
            );
            let mouse_pos = view_matrix.inverse() * vec4(mouse_pos.x, mouse_pos.y, 0.0, 1.0);
            let mouse_pos = vec2(mouse_pos.x, mouse_pos.y);
            mouse_pos
        };

        for player in self.model.players.values() {
            self.circle_renderer.queue(circle_renderer::Instance {
                i_pos: player.pos,
                i_size: player.size,
                i_color: Color::WHITE,
            });
            if let Some(ref projectile) = player.projectile {
                self.circle_renderer.queue(circle_renderer::Instance {
                    i_pos: projectile.pos,
                    i_size: projectile.size,
                    i_color: Color::WHITE,
                });
            }
        }
        for projectile in self.model.projectiles.values() {
            self.circle_renderer.queue(circle_renderer::Instance {
                i_pos: projectile.pos,
                i_size: projectile.size,
                i_color: Color::WHITE,
            });
        }

        self.circle_renderer.draw(framebuffer, view_matrix, rules);
    }
}
