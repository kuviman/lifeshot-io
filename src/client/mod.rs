use crate::*;

mod background;
mod circle_renderer;
mod model;

use background::Background;
use circle_renderer::CircleRenderer;
use model::*;

pub struct ClientApp {
    context: Rc<geng::Context>,
    client_player_id: Option<Id>,
    circle_renderer: CircleRenderer,
    background: Option<Background>,
    action: Arc<Mutex<Action>>,
    camera_pos: Vec2<f32>,
    recv: Arc<Mutex<Option<ServerMessage>>>,
    model: Model,
    mouse_pos: Vec2<f32>,
    connection: Arc<Mutex<Option<net::client::Connection<ClientMessage>>>>,
    connection_promise: Box<Promise<Output = net::client::Connection<ClientMessage>>>,
    font: geng::Font,
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
                    connection.send(ClientMessage::Action(self.action.lock().unwrap().clone()));
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
            background: None,
            client_player_id: None,
            circle_renderer: CircleRenderer::new(context),
            action,
            camera_pos: vec2(0.0, 0.0),
            recv,
            model: Model::new(),
            connection,
            mouse_pos: vec2(0.0, 0.0),
            connection_promise: Box::new(connection_promise),
            font: geng::Font::new(context, include_bytes!("Simply Rounded Bold.ttf").to_vec())
                .unwrap(),
        }
    }
}

impl geng::App for ClientApp {
    fn update(&mut self, delta_time: f64) {
        {
            let mut recv = self.recv.lock().unwrap();
            if let Some(message) = recv.take() {
                self.client_player_id = Some(message.client_player_id);
                self.model.recv(message);
                if self.background.is_none() {
                    self.background = Some(Background::new(&self.model.rules));
                }
            }
        }
        let rules = &self.model.rules;
        if let Some(background) = &mut self.background {
            background.update(delta_time as f32);
        }
        self.model.update(delta_time as f32);
        {
            let mut connection = self.connection.lock().unwrap();
            if connection.is_none() && self.connection_promise.ready() {
                *connection = Some(self.connection_promise.unwrap());
                use net::Sender;
                connection
                    .as_mut()
                    .unwrap()
                    .send(ClientMessage::Action(self.action.lock().unwrap().clone()));
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
        let mut player_alive = false;
        if let Some(id) = self.client_player_id {
            if let Some(player) = self.model.players.get(&id) {
                self.camera_pos = player.pos;
                player_alive = true;
            }
        }
        let player_alive = player_alive;

        ugli::clear(framebuffer, Some(Color::BLACK), None);
        let framebuffer_size = framebuffer.get_size().map(|x| x as f32);
        let center = framebuffer_size / 2.0;
        let scale = framebuffer_size.y / Self::CAMERA_FOV;

        let view_matrix = Mat4::scale(vec3(framebuffer_size.y / framebuffer_size.x, 1.0, 1.0))
            * Mat4::scale_uniform(2.0 / Self::CAMERA_FOV)
            * Mat4::translate(-self.camera_pos.extend(0.0));
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

        if let Some(background) = &self.background {
            background.draw(&mut self.circle_renderer);
        }

        if player_alive {
            let dv = (self.mouse_pos - self.camera_pos).normalize() * Self::CAMERA_FOV;
            const N: usize = 40;
            for i in 1..=N {
                self.circle_renderer.queue(circle_renderer::Instance {
                    i_pos: self.camera_pos + dv * i as f32 / N as f32,
                    i_color: Color::rgba(0.5, 0.5, 1.0, 0.4),
                    i_size: 0.1,
                });
            }
        }

        for food in &self.model.food {
            self.circle_renderer.queue(circle_renderer::Instance {
                i_pos: food.pos,
                i_size: food.size,
                i_color: Color::GREEN,
            });
        }

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

        for player in self.model.players.values() {
            let dv = rules.normalize_delta(player.pos - self.camera_pos);
            let max_y = Self::CAMERA_FOV / 2.0;
            let max_x = max_y * framebuffer_size.x / framebuffer_size.y;
            if dv.x.abs() > max_x || dv.y.abs() > max_y {
                let mut color = Color::WHITE;
                color.a = 0.5;
                self.circle_renderer.queue(circle_renderer::Instance {
                    i_pos: self.camera_pos + vec2(clamp_abs(dv.x, max_x), clamp_abs(dv.y, max_y)),
                    i_color: color,
                    i_size: player.size,
                });
            }
        }

        for spark in &self.model.sparks {
            self.circle_renderer.queue(circle_renderer::Instance {
                i_pos: spark.pos,
                i_color: Color::rgba(1.0, 1.0, 1.0, (1.0 - spark.t / Spark::TIME) * 0.5),
                i_size: spark.size,
            })
        }

        self.circle_renderer.draw(framebuffer, view_matrix, rules);

        if !player_alive {
            let font = &self.font;
            let scale = framebuffer_size.y / 20.0;
            let mid = framebuffer_size / 2.0;
            font.draw_aligned(
                framebuffer,
                "WASD to move",
                vec2(0.0, 5.0 * scale) + mid,
                0.5,
                scale * 2.0,
                Color::rgb(0.5, 0.5, 0.5),
            );
            font.draw_aligned(
                framebuffer,
                "LMB to shoot",
                vec2(0.0, 3.0 * scale) + mid,
                0.5,
                scale * 2.0,
                Color::rgb(0.5, 0.5, 0.5),
            );
            font.draw_aligned(
                framebuffer,
                "F to toggle fullscreen",
                vec2(0.0, 2.0 * scale) + mid,
                0.5,
                scale,
                Color::rgb(0.5, 0.5, 0.5),
            );
            font.draw_aligned(
                framebuffer,
                "Press R to spawn",
                vec2(0.0, -4.0 * scale) + mid,
                0.5,
                scale * 2.0,
                Color::rgb(1.0, 1.0, 1.0),
            );
        }
    }
    fn handle_event(&mut self, event: geng::Event) {
        match event {
            geng::Event::KeyDown { key } => match key {
                geng::Key::R => {
                    let mut connection = self.connection.lock().unwrap();
                    if let Some(connection) = connection.deref_mut() {
                        use net::Sender;
                        connection.send(ClientMessage::Spawn);
                    }
                }
                geng::Key::F => {
                    self.context.window().toggle_fullscreen();
                }
                _ => {}
            },
            _ => {}
        }
    }
}
