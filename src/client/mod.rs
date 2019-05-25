use crate::*;

mod background;
mod circle_renderer;
mod model;

use background::Background;
use circle_renderer::CircleRenderer;
use model::*;

#[derive(geng::Assets)]
pub struct Assets {
    #[path = "aim.wav"]
    aim_sound: geng::Sound,
    #[path = "death.wav"]
    death_sound: geng::Sound,
    #[path = "heal.wav"]
    heal_sound: geng::Sound,
    #[path = "hit.wav"]
    hit_sound: geng::Sound,
    #[path = "shoot.wav"]
    shoot_sound: geng::Sound,
    #[path = "music.ogg"]
    music: geng::Sound,
}
enum ClientAppState {
    Connecting(Box<Promise<Output = net::client::Connection<ServerMessage, ClientMessage>>>),
    Playing(ClientPlayApp),
}

pub struct ClientApp {
    geng: Rc<Geng>,
    assets: Option<Assets>,
    state: Option<ClientAppState>,
}

impl ClientApp {
    pub fn new(geng: &Rc<Geng>, net_opts: NetOpts, assets: Assets) -> Self {
        Self {
            geng: geng.clone(),
            assets: Some(assets),
            state: Some(ClientAppState::Connecting(Box::new(net::client::connect(
                &net_opts.host,
                net_opts.port,
            )))),
        }
    }
}

impl geng::App for ClientApp {
    fn update(&mut self, delta_time: f64) {
        match self.state.as_mut().unwrap() {
            ClientAppState::Connecting(promise) => {
                if promise.ready() {
                    let connection = promise.unwrap();
                    self.state
                        .replace(ClientAppState::Playing(ClientPlayApp::new(
                            &self.geng,
                            connection,
                            self.assets.take().unwrap(),
                        )));
                }
            }
            ClientAppState::Playing(app) => {
                app.update(delta_time);
            }
        }
    }
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        match self.state.as_mut().unwrap() {
            ClientAppState::Connecting(_) => {}
            ClientAppState::Playing(app) => {
                app.draw(framebuffer);
            }
        }
    }
    fn handle_event(&mut self, event: geng::Event) {
        match self.state.as_mut().unwrap() {
            ClientAppState::Connecting(_) => {}
            ClientAppState::Playing(app) => {
                app.handle_event(event);
            }
        }
    }
}

struct SoundPlayerImpl {
    rules: Rules,
    pos: Cell<Vec2<f32>>,
}

pub struct SoundPlayer {
    inner: Rc<SoundPlayerImpl>,
}

struct SoundEffect {
    player: Rc<SoundPlayerImpl>,
    inner: geng::SoundEffect,
}

impl SoundEffect {
    fn set_pos(&mut self, pos: Vec2<f32>) {
        self.inner.set_volume(f64::from(
            clamp(
                1.0 - (self
                    .player
                    .rules
                    .normalize_delta(self.player.pos.get() - pos)
                    .len()
                    / ClientPlayApp::CAMERA_FOV)
                    .powf(2.0),
                0.0..=1.0,
            ) * 0.2,
        ));
    }
}

impl SoundPlayer {
    fn new() -> Self {
        Self {
            inner: Rc::new(SoundPlayerImpl {
                pos: Cell::new(vec2(0.0, 0.0)),
                rules: default(), // TODO
            }),
        }
    }
    fn play(&self, sound: &geng::Sound, pos: Vec2<f32>) -> SoundEffect {
        let mut effect = SoundEffect {
            player: self.inner.clone(),
            inner: sound.effect(),
        };
        effect.set_pos(pos);
        effect.inner.play();
        effect
    }
}

struct ClientPlayApp {
    geng: Rc<Geng>,
    sound_player: Rc<SoundPlayer>,
    assets: Rc<Assets>,
    client_player_id: Option<Id>,
    circle_renderer: CircleRenderer,
    background: Option<Background>,
    action: Action,
    camera_pos: Vec2<f32>,
    model: Model,
    mouse_pos: Vec2<f32>,
    connection: net::client::Connection<ServerMessage, ClientMessage>,
    font: geng::Font,
    music: Option<geng::SoundEffect>,
}

impl ClientPlayApp {
    const CAMERA_FOV: f32 = 30.0;

    pub fn new(
        geng: &Rc<Geng>,
        mut connection: net::client::Connection<ServerMessage, ClientMessage>,
        mut assets: Assets,
    ) -> Self {
        assets.music.looped = true;
        let assets = Rc::new(assets);
        let sound_player = Rc::new(SoundPlayer::new());
        let action = Action::default();
        connection.send(ClientMessage::Action(action.clone()));
        Self {
            geng: geng.clone(),
            sound_player: sound_player.clone(),
            assets: assets.clone(),
            background: None,
            client_player_id: None,
            circle_renderer: CircleRenderer::new(geng),
            action,
            camera_pos: vec2(0.0, 0.0),
            model: Model::new(&assets, &sound_player),
            connection,
            mouse_pos: vec2(0.0, 0.0),
            font: geng::Font::new(geng, include_bytes!("Simply Rounded Bold.ttf").to_vec())
                .unwrap(),
            music: None,
        }
    }
}

impl geng::App for ClientPlayApp {
    fn update(&mut self, delta_time: f64) {
        self.sound_player.inner.pos.set(self.camera_pos);
        {
            let mut got = false;
            for message in self.connection.new_messages() {
                got = true;
                self.client_player_id = Some(message.client_player_id);
                self.model.recv(message);
                if self.background.is_none() {
                    self.background = Some(Background::new(&self.model.rules));
                }
            }
            if got {
                self.connection
                    .send(ClientMessage::Action(self.action.clone()));
            }
        }
        let rules = &self.model.rules;
        if let Some(background) = &mut self.background {
            background.update(delta_time as f32);
        }
        self.model.update(delta_time as f32);
        {
            let mut action = &mut self.action;
            action.target_vel = vec2(0.0, 0.0);
            if self.geng.window().is_key_pressed(geng::Key::W) {
                action.target_vel.y += 1.0;
            }
            if self.geng.window().is_key_pressed(geng::Key::A) {
                action.target_vel.x -= 1.0;
            }
            if self.geng.window().is_key_pressed(geng::Key::S) {
                action.target_vel.y -= 1.0;
            }
            if self.geng.window().is_key_pressed(geng::Key::D) {
                action.target_vel.x += 1.0;
            }
            action.shoot = self
                .geng
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
            let mouse_pos = self.geng.window().mouse_pos().map(|x| x as f32);
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
            player.draw(self.client_player_id, &mut self.circle_renderer);
        }
        for projectile in self.model.projectiles.values() {
            self.circle_renderer.queue(circle_renderer::Instance {
                i_pos: projectile.pos,
                i_size: projectile.size,
                i_color: if Some(projectile.owner_id) == self.client_player_id {
                    Color::rgb(0.5, 0.5, 1.0)
                } else {
                    Color::rgb(1.0, 0.5, 0.5)
                },
            });
        }

        for player in self.model.players.values() {
            let dv = rules.normalize_delta(player.pos - self.camera_pos);
            let max_y = Self::CAMERA_FOV / 2.0;
            let max_x = max_y * framebuffer_size.x / framebuffer_size.y;
            if dv.x.abs() > max_x || dv.y.abs() > max_y {
                self.circle_renderer.queue(circle_renderer::Instance {
                    i_pos: self.camera_pos + vec2(clamp_abs(dv.x, max_x), clamp_abs(dv.y, max_y)),
                    i_color: Color::rgba(1.0, 0.5, 0.5, 0.5),
                    i_size: player.size,
                });
            }
        }

        for spark in &self.model.sparks {
            self.circle_renderer.queue(circle_renderer::Instance {
                i_pos: spark.pos,
                i_color: {
                    let mut color = spark.color;
                    color.a = (1.0 - spark.t / Spark::TIME) * 0.5;
                    color
                },
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
                "This is a work in progress",
                vec2(0.0, 9.0 * scale) + mid,
                0.5,
                scale * 0.3,
                Color::rgb(0.5, 0.5, 0.5),
            );
            font.draw_aligned(
                framebuffer,
                "You can report bugs and suggest features on the issue tracker",
                vec2(0.0, 8.7 * scale) + mid,
                0.5,
                scale * 0.3,
                Color::rgb(0.5, 0.5, 0.5),
            );
            font.draw_aligned(
                framebuffer,
                "(link to the repo in top right corner)",
                vec2(0.0, 8.4 * scale) + mid,
                0.5,
                scale * 0.3,
                Color::rgb(0.5, 0.5, 0.5),
            );

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
                    self.connection.send(ClientMessage::Spawn);
                    if self.music.is_none() {
                        self.music = Some({
                            let mut music = self.assets.music.play();
                            music.set_volume(0.2);
                            music
                        })
                    }
                }
                geng::Key::F => {
                    self.geng.window().toggle_fullscreen();
                }
                _ => {}
            },
            _ => {}
        }
    }
}
