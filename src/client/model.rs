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
    fn recv(
        &mut self,
        e: common_model::Entity,
        target_vel: Option<(Vec2<f32>, f32)>,
        sync_delay: f32,
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
    fn recv(&mut self, p: common_model::Projectile, sync_delay: f32) {
        self.entity.recv(p.entity, None, sync_delay);
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
    fn recv(&mut self, p: common_model::Player, sync_delay: f32) {
        self.entity.recv(
            p.entity,
            Some((
                p.action.target_vel.clamp(1.0) * common_model::Player::MAX_SPEED,
                common_model::Player::ACCELERATION,
            )),
            sync_delay,
        );
        self.projectile = p.projectile.map(|p| Projectile::new(p));
    }
}

pub struct Model {
    pub last_sync_time: Option<f32>,
    pub players: HashMap<Id, Player>,
    pub projectiles: HashMap<Id, Projectile>,
}

impl Model {
    pub fn new() -> Self {
        Self {
            last_sync_time: None,
            players: HashMap::new(),
            projectiles: HashMap::new(),
        }
    }
    pub fn update(&mut self, delta_time: f32) {
        for player in self.players.values_mut() {
            player.update(delta_time);
        }
        for projectile in self.projectiles.values_mut() {
            projectile.update(delta_time);
        }
    }
    pub fn recv(&mut self, mut message: ServerMessage) {
        let sync_delay = if let Some(time) = self.last_sync_time {
            message.model.current_time - time
        } else {
            0.0
        };
        self.last_sync_time = Some(message.model.current_time);

        let mut dead_players: HashSet<Id> = self.players.keys().cloned().collect();
        for player in self.players.values_mut() {
            if let Some(upd) = message.model.players.remove(&player.id) {
                dead_players.remove(&player.id);
                player.recv(upd, sync_delay);
            }
        }
        for player in dead_players {
            self.players.remove(&player);
        }
        for (id, p) in message.model.players {
            self.players.insert(id, Player::new(p));
        }

        let mut dead_projectiles: HashSet<Id> = self.projectiles.keys().cloned().collect();
        for projectile in self.projectiles.values_mut() {
            if let Some(upd) = message.model.projectiles.remove(&projectile.id) {
                dead_projectiles.remove(&projectile.id);
                projectile.recv(upd, sync_delay);
            }
        }
        for projectile in dead_projectiles {
            self.projectiles.remove(&projectile);
        }
        for (id, p) in message.model.projectiles {
            self.projectiles.insert(id, Projectile::new(p));
        }
    }
}
