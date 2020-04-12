use crate::*;

mod background;
mod circle_renderer;
mod model;
mod texture_renderer;

use background::Background;
use circle_renderer::CircleRenderer;
use model::*;
use texture_renderer::TextureRenderer;

#[derive(geng::Assets)]
pub struct Assets {
    #[asset(path = "aim.wav")]
    aim_sound: geng::Sound,
    #[asset(path = "death.wav")]
    death_sound: geng::Sound,
    #[asset(path = "heal.wav")]
    heal_sound: geng::Sound,
    #[asset(path = "hit.wav")]
    hit_sound: geng::Sound,
    #[asset(path = "shoot.wav")]
    shoot_sound: geng::Sound,
    #[asset(path = "music.ogg")]
    music: geng::Sound,
}

pub struct ClientApp {
    geng: Rc<Geng>,
    sound_player: Rc<SoundPlayer>,
    assets: Rc<Assets>,
    client_player_id: Option<Id>,
    circle_renderer: CircleRenderer,
    texture_renderer: TextureRenderer,
    background: Option<Background>,
    action: Action,
    camera_pos: Vec2<f32>,
    model: Model,
    player_names: HashMap<Id, (String, ugli::Texture)>,
    mouse_pos: Vec2<f32>,
    connection: net::client::Connection<ServerMessage, ClientMessage>,
    traffic_watch: TrafficWatch,
    ping_watch: PingWatch,
    font: geng::Font,
    music: Option<geng::SoundEffect>,
    ui_state: UiState,
    ui_controller: geng::ui::Controller,
}

impl ClientApp {
    pub fn run(opts: &Opts, net_opts: &NetOpts) {
        let geng = Rc::new(Geng::new(geng::ContextOptions {
            title: "lifeshot.io".to_owned(),
            ..default()
        }));
        let name = opts.name.clone();
        let connection_future = net::client::connect(&net_opts.addr);
        let assets_future = <Assets as geng::LoadAsset>::load(&geng, ".");
        let app = geng::LoadingScreen::new(
            &geng,
            geng::EmptyLoadingScreen,
            future::join(assets_future, connection_future),
            {
                let geng = geng.clone();
                move |(assets, mut connection)| {
                    connection.send(ClientMessage::SetName(name));
                    Self::new(&geng, connection, assets.unwrap())
                }
            },
        );
        geng::run(geng, app);
    }
}

struct SoundPlayerImpl {
    rules: Rules,
    pos: Cell<Vec2<f32>>,
    volume: Cell<f64>,
}

pub struct SoundPlayer {
    inner: Rc<SoundPlayerImpl>,
}

pub struct SoundEffect {
    player: Rc<SoundPlayerImpl>,
    inner: geng::SoundEffect,
}

impl Deref for SoundEffect {
    type Target = geng::SoundEffect;
    fn deref(&self) -> &geng::SoundEffect {
        &self.inner
    }
}

impl DerefMut for SoundEffect {
    fn deref_mut(&mut self) -> &mut geng::SoundEffect {
        &mut self.inner
    }
}

impl SoundEffect {
    fn set_pos(&mut self, pos: Vec2<f32>) {
        self.inner.set_volume(
            f64::from(clamp(
                1.0 - (self
                    .player
                    .rules
                    .normalize_delta(self.player.pos.get() - pos)
                    .len()
                    / ClientApp::CAMERA_FOV)
                    .powf(2.0),
                0.0..=1.0,
            )) * self.player.volume.get(),
        );
    }
}

impl SoundPlayer {
    fn new() -> Self {
        Self {
            inner: Rc::new(SoundPlayerImpl {
                pos: Cell::new(vec2(0.0, 0.0)),
                rules: default(), // TODO
                volume: Cell::new(0.5),
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

struct TrafficWatch {
    inbound: (usize, usize),
    outbound: (usize, usize),
    timer: Timer,
}

impl TrafficWatch {
    fn new() -> Self {
        Self {
            inbound: (0, 0),
            outbound: (0, 0),
            timer: Timer::new(),
        }
    }
    fn update(&mut self, traffic: &net::Traffic) {
        if self.timer.elapsed() > 1.0 {
            fn fmt((prev, cur): (usize, usize)) -> String {
                format!("{}KB/s", (cur - prev) / 1024)
            }
            debug!("in: {}, out: {}", fmt(self.inbound), fmt(self.outbound));
            self.timer.tick();
            self.inbound.0 = traffic.inbound();
            self.outbound.0 = traffic.outbound();
        }
        self.inbound.1 = traffic.inbound();
        self.outbound.1 = traffic.outbound();
    }
}

struct PingWatch {
    min: f64,
    max: f64,
    ping_timer: Timer,
    timer: Timer,
    text: String,
}

impl PingWatch {
    fn new() -> Self {
        Self {
            min: 100.0,
            max: 0.0,
            ping_timer: Timer::new(),
            timer: Timer::new(),
            text: "ping".to_owned(),
        }
    }
    fn update(&mut self) {
        let ping = self.ping_timer.tick();
        self.min = partial_min(self.min, ping);
        self.max = partial_max(self.max, ping);
        if self.timer.elapsed() > 1.0 {
            self.text = format!(
                "ping: {}-{}ms",
                (self.min * 1000.0) as i32,
                (self.max * 1000.0) as i32
            );
            self.timer.tick();
            self.min = 100.0;
            self.max = 0.0;
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Settings {
    volume: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self { volume: 0.5 }
    }
}

struct UiState {
    geng: Rc<Geng>,
    settings: AutoSave<Settings>,
    volume_slider: geng::ui::Slider,
}

impl UiState {
    fn new(geng: &Rc<Geng>) -> Self {
        let ui_theme = Rc::new(geng::ui::Theme::default(geng));
        Self {
            geng: geng.clone(),
            settings: AutoSave::load(".settings"),
            volume_slider: geng::ui::Slider::new(geng, &ui_theme),
        }
    }
    fn volume(&self) -> f64 {
        return self.settings.volume * 0.2;
    }
    fn ui<'a>(&'a mut self) -> impl geng::ui::Widget + 'a {
        use geng::ui;
        use geng::ui::*;
        let settings = &mut self.settings;
        let current_volume = settings.volume;
        ui::row![
            geng::ui::text("volume", self.geng.default_font(), 24.0, Color::WHITE)
                .padding_right(24.0),
            self.volume_slider
                .ui(
                    current_volume,
                    0.0..=1.0,
                    Box::new(move |new_value| {
                        settings.volume = new_value;
                    })
                )
                .fixed_size(vec2(100.0, 24.0)),
        ]
        .padding_bottom(24.0)
        .padding_right(24.0)
        .align(vec2(1.0, 0.0))
    }
}

impl ClientApp {
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
            texture_renderer: TextureRenderer::new(geng),
            action,
            camera_pos: vec2(0.0, 0.0),
            model: Model::new(&assets, &sound_player),
            player_names: HashMap::new(),
            connection,
            traffic_watch: TrafficWatch::new(),
            ping_watch: PingWatch::new(),
            mouse_pos: vec2(0.0, 0.0),
            font: geng::Font::new(
                geng,
                include_bytes!("../../static/Simply Rounded Bold.ttf").to_vec(),
            )
            .unwrap(),
            music: None,
            ui_state: UiState::new(geng),
            ui_controller: geng::ui::Controller::new(),
        }
    }
}

impl geng::State for ClientApp {
    fn update(&mut self, delta_time: f64) {
        self.ui_controller.update(self.ui_state.ui(), delta_time);
        self.sound_player.inner.volume.set(self.ui_state.volume());
        if let Some(music) = &mut self.music {
            music.set_volume(self.ui_state.volume());
        }
        self.traffic_watch.update(self.connection.traffic());
        self.sound_player.inner.pos.set(self.camera_pos);
        {
            let mut got = false;
            for message in self.connection.new_messages() {
                got = true;
                self.client_player_id = Some(message.client_player_id);
                for event in &message.events {
                    if let common_model::Event::PlayerName { player_id, name } = event {
                        let name = if name.is_empty() {
                            "<noname>"
                        } else {
                            name.as_str()
                        };
                        let height = 32usize;
                        let width = self.font.measure(name, height as f32).width().ceil() as usize;
                        let mut texture =
                            ugli::Texture::new_uninitialized(self.geng.ugli(), vec2(width, height));
                        {
                            let mut framebuffer = ugli::Framebuffer::new_color(
                                self.geng.ugli(),
                                ugli::ColorAttachment::Texture(&mut texture),
                            );
                            ugli::clear(
                                &mut framebuffer,
                                Some(Color::rgba(1.0, 1.0, 1.0, 0.0)),
                                None,
                            );
                            self.font.draw(
                                &mut framebuffer,
                                name,
                                vec2(0.0, 0.0),
                                height as f32,
                                Color::rgba(1.0, 1.0, 1.0, 0.7),
                            );
                        }
                        self.player_names
                            .insert(*player_id, (name.to_owned(), texture));
                    }
                }
                self.model.recv(message);
                if self.background.is_none() {
                    self.background = Some(Background::new(&self.model.rules));
                }
            }
            if got {
                self.ping_watch.update();
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
        let framebuffer_size = framebuffer.size().map(|x| x as f32);
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

        for food in self.model.food.values() {
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

        for player in self.model.players.values() {
            if let Some((_, texture)) = self.player_names.get(&player.id) {
                self.texture_renderer
                    .draw(framebuffer, view_matrix, player.pos, texture, rules);
            }
        }

        let font = &self.font;
        let scale = framebuffer_size.y / 20.0;
        let mid = framebuffer_size / 2.0;

        {
            const FONT_SIZE: f32 = 16.0;
            let mut y = framebuffer_size.y - 100.0;
            for (id, scores) in &self.model.scores {
                if let Some((name, _)) = self.player_names.get(id) {
                    y -= FONT_SIZE;
                    font.draw_aligned(
                        framebuffer,
                        &format!("{}: {} kills, {} deaths", name, scores.kills, scores.deaths),
                        vec2(framebuffer_size.x - 100.0, y),
                        1.0,
                        FONT_SIZE,
                        Color::rgba(1.0, 1.0, 1.0, 0.6),
                    );
                }
            }
        }

        if !player_alive {
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

        font.draw(
            framebuffer,
            &self.ping_watch.text,
            vec2(10.0, 10.0),
            16.0,
            Color::rgb(0.5, 0.5, 0.5),
        );

        self.ui_controller.draw(self.ui_state.ui(), framebuffer);
    }
    fn handle_event(&mut self, event: geng::Event) {
        if self
            .ui_controller
            .handle_event(self.ui_state.ui(), event.clone())
        {
            return;
        }
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
