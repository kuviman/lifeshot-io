use crate::*;

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
    fn recv(&mut self, e: common_model::Entity) {
        self.next_pos = e.pos;
        self.next_vel = e.vel;
        self.size = e.size;
        self.delayed = Self::DELAY;
    }
    fn update(&mut self, mut delta_time: f32) {
        if self.delayed > 0.0 {
            let dt = delta_time.min(self.delayed);
            let k = dt / self.delayed;
            self.delayed -= delta_time;
            delta_time -= dt;
            self.pos += (self.next_pos - self.pos) * k;
            self.vel += (self.next_vel - self.vel) * k;
        }
        self.pos += self.vel * delta_time;
    }
}

pub struct Projectile {
    owner_id: Id,
    entity: Entity,
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
    fn new(p: common_model::Projectile) -> Self {
        Self {
            owner_id: p.owner_id,
            entity: Entity::new(p.entity),
        }
    }
}

pub struct Player {
    pub entity: Entity,
    pub projectile: Option<Projectile>,
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
    fn new(p: common_model::Player) -> Self {
        Self {
            entity: Entity::new(p.entity),
            projectile: p.projectile.map(|p| Projectile::new(p)),
        }
    }
    fn recv(&mut self, p: common_model::Player) {
        self.entity.recv(p.entity);
        self.projectile = p.projectile.map(|p| Projectile::new(p));
    }
}

pub struct Model {
    pub players: HashMap<Id, Player>,
    pub projectiles: Vec<Projectile>,
}

impl Model {
    pub fn new() -> Self {
        Self {
            players: HashMap::new(),
            projectiles: Vec::new(),
        }
    }
    pub fn update(&mut self, delta_time: f32) {
        for player in self.players.values_mut() {
            player.update(delta_time);
        }
        for projectile in &mut self.projectiles {
            projectile.update(delta_time);
        }
    }
    pub fn recv(&mut self, mut message: ServerMessage) {
        let mut dead_players: HashSet<Id> = self.players.keys().cloned().collect();
        for player in self.players.values_mut() {
            if let Some(upd) = message.model.players.remove(&player.id) {
                dead_players.remove(&player.id);
                player.recv(upd);
            }
        }
        for player in dead_players {
            self.players.remove(&player);
        }
        for (id, p) in message.model.players {
            self.players.insert(id, Player::new(p));
        }
        self.projectiles = message
            .model
            .projectiles
            .into_iter()
            .map(|p| Projectile::new(p))
            .collect();
    }
}
