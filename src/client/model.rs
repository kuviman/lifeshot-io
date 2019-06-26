use super::*;

pub struct Entity {
    pub id: Id,
    pub pos: Vec2<f32>,
    pub next_pos: Vec2<f32>,
    pub vel: Vec2<f32>,
    pub next_vel: Vec2<f32>,
    pub size: f32,
    pub delayed: f32,
}

impl Entity {
    const DELAY: f32 = 0.1;
    pub fn new(e: common_model::Entity) -> Self {
        Self {
            id: e.id,
            pos: e.pos,
            next_pos: e.pos,
            vel: e.vel,
            next_vel: e.vel,
            size: e.size,
            delayed: 0.0,
        }
    }
    fn mass(&self) -> f32 {
        self.size * self.size
    }
    fn recv(
        &mut self,
        e: common_model::Entity,
        target_vel: Option<(Vec2<f32>, f32)>,
        sync_delay: f32,
        rules: &Rules,
    ) {
        self.size = e.size;
        self.delayed = Self::DELAY;
        let next_time = self.delayed + sync_delay;
        if let Some((target_vel, acceleration)) = target_vel {
            let time_to_target_vel = (target_vel - e.vel).len() / acceleration;
            let t = time_to_target_vel.min(next_time);
            let t_vel = e.vel + (target_vel - e.vel).clamp(acceleration * t);
            self.next_pos = e.pos + (e.vel + t_vel) / 2.0 * t;
            self.next_pos += t_vel * (next_time - t);
            self.next_vel = t_vel;
        } else {
            self.next_pos = e.pos + e.vel * next_time;
            self.next_vel = e.vel;
        }
    }
    fn update(&mut self, mut delta_time: f32, rules: &Rules) {
        if self.delayed > 0.0 {
            let next_pos_using_vel = self.pos + (self.vel + self.next_vel) / 2.0 * self.delayed;
            let dt = delta_time.min(self.delayed);
            let k = dt / self.delayed;
            self.delayed -= delta_time;
            delta_time -= dt;
            let next_vel = self.vel + (self.next_vel - self.vel) * k;
            self.pos += (self.vel + next_vel) / 2.0 * dt;

            self.pos += rules.normalize_delta(self.next_pos - next_pos_using_vel) * k;
            self.vel = next_vel;
        }
        self.pos = rules.normalize_pos(self.pos + self.vel * delta_time);
    }
}

pub struct Projectile {
    pub owner_id: Id,
    entity: Entity,
    next_spark: f32,
}

impl Deref for Projectile {
    type Target = Entity;
    fn deref(&self) -> &Entity {
        &self.entity
    }
}

impl DerefMut for Projectile {
    fn deref_mut(&mut self) -> &mut Entity {
        &mut self.entity
    }
}

impl Projectile {
    const SPARK_FREQ: f32 = 500.0;
    fn new(p: common_model::Projectile) -> Self {
        Self {
            owner_id: p.owner_id,
            entity: Entity::new(p.entity),
            next_spark: 0.0,
        }
    }
    fn recv(&mut self, p: common_model::Projectile, sync_delay: f32, rules: &Rules) {
        self.entity.recv(p.entity, None, sync_delay, rules);
    }
    fn update_sparks(
        &mut self,
        client_player_id: Option<Id>,
        delta_time: f32,
        sparks: &mut Vec<Spark>,
    ) {
        self.next_spark -= delta_time * self.entity.mass();
        while self.next_spark < 0.0 {
            self.next_spark += 1.0 / Self::SPARK_FREQ;
            sparks.push(Spark::new(
                &self.entity,
                if Some(self.owner_id) == client_player_id {
                    Color::rgb(0.5, 0.5, 1.0)
                } else {
                    Color::rgb(1.0, 0.5, 0.5)
                },
            ));
        }
    }
}

pub struct Spark {
    pub pos: Vec2<f32>,
    pub size: f32,
    pub vel: Vec2<f32>,
    pub color: Color<f32>,
    pub t: f32,
}

impl Spark {
    pub const TIME: f32 = 0.3;
    const MAX_SPEED: f32 = 5.0;
    pub fn new(e: &Entity, color: Color<f32>) -> Self {
        Self {
            pos: e.pos,
            size: global_rng().gen_range(e.size / 2.0, e.size),
            vel: distributions::UnitCircleInside.sample(&mut global_rng()) * Self::MAX_SPEED,
            color,
            t: 0.0,
        }
    }
    fn update(&mut self, delta_time: f32) {
        self.pos += self.vel * delta_time;
        self.t += delta_time;
    }
    fn alive(&self) -> bool {
        self.t < Self::TIME
    }
}

pub struct Player {
    sound_player: Rc<SoundPlayer>,
    assets: Rc<Assets>,
    pub action: Action,
    pub entity: Entity,
    pub projectile: Option<(Projectile, SoundEffect)>,
    time: f32,
}

impl Deref for Player {
    type Target = Entity;
    fn deref(&self) -> &Entity {
        &self.entity
    }
}

impl DerefMut for Player {
    fn deref_mut(&mut self) -> &mut Entity {
        &mut self.entity
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        if let Some((_, sound_effect)) = &mut self.projectile {
            sound_effect.pause();
        }
    }
}

impl Player {
    fn new(p: common_model::Player, sound_player: &Rc<SoundPlayer>, assets: &Rc<Assets>) -> Self {
        Self {
            assets: assets.clone(),
            sound_player: sound_player.clone(),
            action: p.action,
            entity: Entity::new(p.entity),
            projectile: p.projectile.map(|p| {
                let sound_effect = sound_player.play(&assets.aim_sound, p.pos);
                (Projectile::new(p), sound_effect)
            }),
            time: 0.0,
        }
    }
    fn recv(&mut self, p: common_model::Player, sync_delay: f32, rules: &Rules) {
        self.action = p.action.clone();
        self.entity.recv(
            p.entity,
            Some((
                {
                    let mut target_vel =
                        p.action.target_vel.clamp(1.0) * common_model::Player::MAX_SPEED;
                    if self.action.shoot {
                        target_vel = target_vel.clamp(common_model::Player::MAX_AIMING_SPEED);
                    }
                    target_vel
                },
                common_model::Player::ACCELERATION,
            )),
            sync_delay,
            rules,
        );
        if let Some(p) = p.projectile {
            if let Some((projectile, sound_effect)) = &mut self.projectile {
                sound_effect.set_pos(p.pos);
                *projectile = Projectile::new(p);
            } else {
                self.projectile = Some({
                    let sound_effect = self.sound_player.play(&self.assets.aim_sound, p.pos);
                    (Projectile::new(p), sound_effect)
                });
            }
        } else {
            if let Some((_, mut sound_effect)) = self.projectile.take() {
                sound_effect.pause();
            }
        }
    }
    fn update(&mut self, delta_time: f32, rules: &Rules) {
        self.time += delta_time;
        if let Some((projectile, _)) = &mut self.projectile {
            projectile.pos = self.entity.pos
                + rules
                    .normalize_delta(self.action.aim - self.entity.pos)
                    .clamp(self.entity.size);
        }
        self.entity.update(delta_time, rules);
    }
    pub fn draw(&self, client_player_id: Option<Id>, renderer: &mut CircleRenderer) {
        renderer.queue(circle_renderer::Instance {
            i_pos: self.pos,
            i_size: self.size,
            i_color: if Some(self.id) == client_player_id {
                Color::BLUE
            } else {
                Color::RED
            },
        });
        renderer.queue(circle_renderer::Instance {
            i_pos: self.pos,
            i_size: self.size * 0.9,
            i_color: Color::rgba(1.0, 1.0, 1.0, 0.2),
        });
        renderer.queue(circle_renderer::Instance {
            i_pos: self.pos,
            i_size: self.size * 0.9 * {
                ((self.time * 5.0).sin() * 0.5 + 0.5).powf(3.0) * 0.5 + 0.5
            },
            i_color: Color::rgba(1.0, 1.0, 1.0, 0.1),
        });
        if let Some((ref projectile, _)) = self.projectile {
            renderer.queue(circle_renderer::Instance {
                i_pos: projectile.pos,
                i_size: projectile.size,
                i_color: if Some(projectile.owner_id) == client_player_id {
                    Color::rgb(0.5, 0.5, 1.0)
                } else {
                    Color::rgb(1.0, 0.5, 0.5)
                },
            });
        }
    }
}

pub struct Model {
    assets: Rc<Assets>,
    sound_player: Rc<SoundPlayer>,
    pub scores: HashMap<Id, common_model::Scores>,
    pub last_sync_time: Option<f32>,
    pub client_player_id: Option<Id>,
    pub rules: Rules,
    pub players: HashMap<Id, Player>,
    pub projectiles: HashMap<Id, Projectile>,
    pub food: HashMap<Id, common_model::Food>,
    pub sparks: Vec<Spark>,
}

impl Model {
    pub fn new(assets: &Rc<Assets>, sound_player: &Rc<SoundPlayer>) -> Self {
        Self {
            assets: assets.clone(),
            scores: HashMap::new(),
            sound_player: sound_player.clone(),
            last_sync_time: None,
            rules: default(),
            players: HashMap::new(),
            projectiles: HashMap::new(),
            food: HashMap::new(),
            sparks: Vec::new(),
            client_player_id: None,
        }
    }
    pub fn update(&mut self, delta_time: f32) {
        let rules = &self.rules;
        for player in self.players.values_mut() {
            player.update(delta_time, rules);
            if let Some((projectile, _)) = &mut player.projectile {
                projectile.update_sparks(self.client_player_id, delta_time, &mut self.sparks);
            }
        }
        for projectile in self.projectiles.values_mut() {
            projectile.update(delta_time, rules);
            projectile.update_sparks(self.client_player_id, delta_time, &mut self.sparks);
        }
        for spark in &mut self.sparks {
            spark.update(delta_time);
        }
        self.sparks.retain(|e| e.alive());
    }
    pub fn recv(&mut self, mut message: ServerMessage) {
        self.client_player_id = Some(message.client_player_id);
        self.rules = message.model.rules;
        let rules = &self.rules;
        let sync_delay = if let Some(time) = self.last_sync_time {
            (message.model.current_time - time) / 2.0
        } else {
            0.0
        };
        self.last_sync_time = Some(message.model.current_time);

        let mut dead_players: HashSet<Id> = self.players.keys().cloned().collect();
        for player in self.players.values_mut() {
            if let Some(upd) = message.model.players.remove(&player.id) {
                dead_players.remove(&player.id);
                player.recv(upd, sync_delay, rules);
            }
        }
        for player in dead_players {
            if let Some(player) = self.players.remove(&player) {
                self.sound_player.play(&self.assets.death_sound, player.pos);
            }
        }
        for (id, p) in message.model.players {
            self.players
                .insert(id, Player::new(p, &self.sound_player, &self.assets));
        }

        let mut dead_projectiles: HashSet<Id> = self.projectiles.keys().cloned().collect();
        for projectile in self.projectiles.values_mut() {
            if let Some(upd) = message.model.projectiles.remove(&projectile.id) {
                dead_projectiles.remove(&projectile.id);
                projectile.recv(upd, sync_delay, rules);
            }
        }
        for projectile in dead_projectiles {
            if let Some(p) = self.projectiles.remove(&projectile) {
                self.sound_player.play(&self.assets.hit_sound, p.pos); // TODO: on actual hit
            }
        }
        for (id, p) in message.model.projectiles {
            self.sound_player.play(&self.assets.shoot_sound, p.pos);
            self.projectiles.insert(id, Projectile::new(p));
        }

        for event in message.events {
            match event {
                common_model::Event::Food(event) => match event {
                    common_model::FoodEvent::Add(food) => {
                        self.food.insert(food.id, food);
                    }
                    common_model::FoodEvent::Remove(id) => {
                        if let Some(food) = self.food.remove(&id) {
                            self.sound_player.play(&self.assets.heal_sound, food.pos);
                        }
                    }
                },
                common_model::Event::PlayerName { .. } => {}
                common_model::Event::ScoresUpdate(scores) => {
                    self.scores = scores;
                }
            }
        }
    }
}
