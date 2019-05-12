use crate::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Entity {
    pub pos: Vec2<f32>,
    pub vel: Vec2<f32>,
    pub size: f32,
}

impl Entity {
    fn update(&mut self, delta_time: f32) {
        self.pos += self.vel * delta_time;
    }
    fn alive(&self) -> bool {
        self.size > 0.0
    }
    fn mass(&self) -> f32 {
        self.size * self.size
    }
    fn add_mass(&mut self, delta_mass: f32) {
        let mass = self.mass() + delta_mass;
        self.size = mass.max(0.0).sqrt();
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Action {
    pub target_vel: Vec2<f32>,
    pub shoot: Option<Vec2<f32>>,
}

impl Default for Action {
    fn default() -> Self {
        Self {
            target_vel: vec2(0.0, 0.0),
            shoot: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PlayerId(usize);

impl PlayerId {
    pub fn new() -> Self {
        static NEXT_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
        PlayerId(NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Player {
    pub id: PlayerId,
    entity: Entity,
    pub projectile: Option<Projectile>,
    pub action: Action,
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

impl Player {
    const MAX_SPEED: f32 = 8.0;
    const ACCELERATION: f32 = 15.0;
    const PROJECTILE_MASS_GAIN_SPEED: f32 = 0.3;
    const PROJECTILE_COST_SPEED: f32 = 0.1;
    pub fn new() -> Self {
        Self {
            id: PlayerId::new(),
            projectile: None,
            entity: Entity {
                pos: vec2(0.0, 0.0),
                vel: vec2(0.0, 0.0),
                size: 1.0,
            },
            action: default(),
        }
    }
    fn update(&mut self, delta_time: f32) -> Option<Projectile> {
        self.entity.vel += (self.action.target_vel.clamp(1.0) * Self::MAX_SPEED - self.entity.vel)
            .clamp(Self::ACCELERATION * delta_time);
        self.entity.update(delta_time);

        if let Some(target) = self.action.shoot {
            if self.projectile.is_none() {
                self.projectile = Some(Projectile {
                    owner_id: self.id,
                    entity: Entity {
                        pos: self.entity.pos,
                        vel: vec2(0.0, 0.0),
                        size: 0.0,
                    },
                });
            }
            let projectile = self.projectile.as_mut().unwrap();
            let e = &mut self.entity;

            projectile.pos = e.pos + (target - e.pos).clamp(e.size);
            projectile.vel = (target - e.pos).normalize() * Projectile::SPEED;
            projectile.add_mass(Self::PROJECTILE_MASS_GAIN_SPEED * delta_time);
            e.add_mass(-Self::PROJECTILE_COST_SPEED * delta_time);
            None
        } else {
            self.projectile.take()
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Projectile {
    entity: Entity,
    owner_id: PlayerId,
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
    const SPEED: f32 = 25.0;
    const DEATH_SPEED: f32 = 0.1;
    fn update(&mut self, delta_time: f32) {
        self.size -= Self::DEATH_SPEED * delta_time;
        self.entity.update(delta_time);
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Model {
    pub players: HashMap<PlayerId, Player>,
    pub projectiles: Vec<Projectile>,
}

impl Model {
    pub fn update(&mut self, delta_time: f32) {
        for player in self.players.values_mut() {
            if let Some(projectile) = player.update(delta_time) {
                self.projectiles.push(projectile);
            }
        }
        for projectile in &mut self.projectiles {
            projectile.update(delta_time);
        }
        self.players.retain(|_, e| e.alive());
        self.projectiles.retain(|e| e.alive());
    }
    pub fn handle(&mut self, player_id: PlayerId, message: ClientMessage) {
        if let Some(player) = self.players.get_mut(&player_id) {
            player.action = message.action;
        }
    }
}

impl Default for Model {
    fn default() -> Self {
        Self {
            players: HashMap::new(),
            projectiles: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServerMessage {
    pub model: Model,
}

impl net::Message for ServerMessage {}

#[derive(Serialize, Deserialize)]
pub struct ClientMessage {
    pub action: Action,
}

impl net::Message for ClientMessage {}
