use crate::*;

pub mod prelude {
    pub use super::{Action, ClientMessage, Id, Rules, ServerMessage};
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Entity {
    pub id: Id,
    pub pos: Vec2<f32>,
    pub vel: Vec2<f32>,
    pub size: f32,
}

impl Entity {
    fn update(&mut self, delta_time: f32, rules: &Rules) {
        self.pos = rules.normalize_pos(self.pos + self.vel * delta_time);
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
    pub fn collide(a: &mut Self, b: &mut Self, rules: &Rules) {
        let dist = rules.normalize_delta(a.pos - b.pos).len();
        if dist < 1e-3 {
            return;
        }
        let penetration = (a.size + b.size) - dist;
        let penetration = penetration.min(min(a.size, b.size));
        let n = rules.normalize_delta(b.pos - a.pos).normalize();
        if penetration > 0.0 {
            let ka = 1.0 / a.mass();
            let kb = 1.0 / b.mass();
            let sum_k = ka + kb;
            let ka = ka / sum_k;
            let kb = kb / sum_k;
            a.pos -= n * penetration * ka;
            b.pos += n * penetration * kb;
        }
    }
    pub fn hit(&mut self, target: &mut Self, k: f32, rules: &Rules) -> bool {
        let penetration =
            (self.size + target.size) - rules.normalize_delta(self.pos - target.pos).len();
        let penetration = penetration.min(min(self.size, target.size));
        if penetration > 0.0 {
            let prev_mass = self.mass();
            self.size = (self.size - penetration).max(0.0);
            let delta_mass = prev_mass - self.mass();
            let prev_target_mass = target.mass();
            target.add_mass(-delta_mass * k);
            let real_delta_mass = (prev_target_mass - target.mass()) / k;
            self.add_mass(delta_mass - real_delta_mass);
            true
        } else {
            false
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Action {
    pub target_vel: Vec2<f32>,
    pub shoot: bool,
    pub aim: Vec2<f32>,
}

impl Default for Action {
    fn default() -> Self {
        Self {
            target_vel: vec2(0.0, 0.0),
            shoot: false,
            aim: vec2(0.0, 0.0),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Id(usize);

impl Id {
    pub fn new() -> Self {
        static NEXT_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
        Id(NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Player {
    pub entity: Entity,
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
    pub const INITIAL_SIZE: f32 = 1.0;
    pub const MAX_SPEED: f32 = 8.0;
    pub const ACCELERATION: f32 = 15.0;
    pub const PROJECTILE_MASS_GAIN_SPEED: f32 = 0.3;
    pub const PROJECTILE_COST_SPEED: f32 = 0.1;
    pub const DEATH_SPEED: f32 = 1.0 / 20.0;
    pub fn new(id: Id, pos: Vec2<f32>) -> Self {
        Self {
            projectile: None,
            entity: Entity {
                id,
                pos,
                vel: vec2(0.0, 0.0),
                size: Self::INITIAL_SIZE,
            },
            action: default(),
        }
    }
    fn update(&mut self, delta_time: f32, rules: &Rules) -> Option<Projectile> {
        self.add_mass(-Self::DEATH_SPEED * delta_time);

        self.entity.vel += (self.action.target_vel.clamp(1.0) * Self::MAX_SPEED - self.entity.vel)
            .clamp(Self::ACCELERATION * delta_time);
        self.entity.update(delta_time, rules);

        if self.action.shoot {
            if self.projectile.is_none() {
                self.projectile = Some(Projectile {
                    owner_id: self.id,
                    entity: Entity {
                        id: Id::new(),
                        pos: self.entity.pos,
                        vel: vec2(0.0, 0.0),
                        size: 0.0,
                    },
                });
            }
            let projectile = self.projectile.as_mut().unwrap();
            let me = &mut self.entity;

            projectile.add_mass(Self::PROJECTILE_MASS_GAIN_SPEED * delta_time);
            me.add_mass(-Self::PROJECTILE_COST_SPEED * delta_time);
        }

        if let Some(ref mut projectile) = self.projectile {
            let mut dr = rules.normalize_delta(self.action.aim - self.entity.pos);
            if dr.len() > self.entity.size {
                dr = dr.normalize();
            }
            projectile.pos = self.entity.pos + dr * self.entity.size;
            projectile.vel = dr * Projectile::SPEED;
        }

        if self.action.shoot {
            None
        } else {
            self.projectile.take()
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Projectile {
    pub entity: Entity,
    pub owner_id: Id,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Food {
    pub entity: Entity,
}

impl Deref for Food {
    type Target = Entity;
    fn deref(&self) -> &Entity {
        &self.entity
    }
}

impl DerefMut for Food {
    fn deref_mut(&mut self) -> &mut Entity {
        &mut self.entity
    }
}

impl Food {
    const SIZE: f32 = 0.1;
    const EFFECIENCY: f32 = 3.0;
    fn new(pos: Vec2<f32>, rules: &Rules) -> Self {
        Self {
            entity: Entity {
                id: Id::new(),
                size: Self::SIZE,
                pos,
                vel: vec2(0.0, 0.0),
            },
        }
    }
}

impl Projectile {
    const SPEED: f32 = 25.0;
    const DEATH_SPEED: f32 = 0.1;
    const STRENGTH: f32 = 0.5;
    fn update(&mut self, delta_time: f32, rules: &Rules) {
        self.add_mass(-Self::DEATH_SPEED * delta_time);
        self.entity.update(delta_time, rules);
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Rules {
    pub world_size: f32,
}

impl Default for Rules {
    fn default() -> Self {
        Self { world_size: 100.0 }
    }
}

impl Rules {
    pub fn normalize_pos(&self, pos: Vec2<f32>) -> Vec2<f32> {
        let mut pos = pos;
        while pos.x > self.world_size {
            pos.x -= self.world_size;
        }
        while pos.x < 0.0 {
            pos.x += self.world_size;
        }
        while pos.y > self.world_size {
            pos.y -= self.world_size;
        }
        while pos.y < 0.0 {
            pos.y += self.world_size;
        }
        pos
    }
    pub fn normalize_delta(&self, v: Vec2<f32>) -> Vec2<f32> {
        let mut v = self.normalize_pos(v);
        if v.x > self.world_size / 2.0 {
            v.x -= self.world_size;
        }
        if v.y > self.world_size / 2.0 {
            v.y -= self.world_size;
        }
        v
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Model {
    pub rules: Rules,
    pub current_time: f32,
    pub players: HashMap<Id, Player>,
    pub projectiles: HashMap<Id, Projectile>,
    pub food: Vec<Food>,
}

impl Model {
    pub const MAX_FOOD_EXTRA: f32 = 5.0;

    pub fn new_player(&mut self) -> Id {
        Id::new()
    }
    pub fn update(&mut self, delta_time: f32) {
        let rules = &self.rules;
        self.current_time += delta_time;
        for player in self.players.values_mut() {
            if let Some(projectile) = player.update(delta_time, rules) {
                self.projectiles.insert(projectile.id, projectile);
            }
        }
        for projectile in self.projectiles.values_mut() {
            projectile.update(delta_time, rules);
        }
        for projectile in self.projectiles.values_mut() {
            for player in self.players.values_mut() {
                if projectile.owner_id != player.id {
                    projectile.hit(player, Projectile::STRENGTH, rules);
                }
            }
        }
        fn iter_pairs<T>(mut v: Vec<&mut T>, mut f: impl FnMut(&mut T, &mut T)) {
            for i in 1..v.len() {
                let (head, tail) = v.split_at_mut(i);
                let first = head.last_mut().unwrap();
                for second in tail {
                    f(first, second);
                }
            }
        }
        iter_pairs(self.players.values_mut().collect(), |p1, p2| {
            Entity::collide(p1, p2, rules);
        });

        let total_mass = self.players.values().map(|p| p.mass()).sum::<f32>()
            + Food::EFFECIENCY * self.food.iter().map(|f| f.mass()).sum::<f32>();
        if total_mass - self.players.len() as f32 * Player::INITIAL_SIZE * Player::INITIAL_SIZE
            < Self::MAX_FOOD_EXTRA
        {
            let pos = vec2(
                global_rng().gen_range(0.0, rules.world_size),
                global_rng().gen_range(0.0, rules.world_size),
            );
            const N: usize = 10;
            let mut n = N;
            for _ in 0..5 {
                n = min(n, global_rng().gen_range(1, N));
            }
            for _ in 0..n {
                self.food.push(Food::new(
                    rules.normalize_pos(
                        pos + vec2(
                            global_rng().gen_range(-1.0, 1.0),
                            global_rng().gen_range(-1.0, 1.0),
                        ) / 5.0,
                    ),
                    rules,
                ));
            }
        }

        for player in self.players.values_mut() {
            for food in &mut self.food {
                if rules.normalize_delta(player.pos - food.pos).len() < player.size + food.size {
                    player.add_mass(food.mass() * Food::EFFECIENCY);
                    food.size = 0.0;
                }
            }
        }

        self.players.retain(|_, e| e.alive());
        self.projectiles.retain(|_, e| e.alive());
        self.food.retain(|e| e.alive());
    }
    pub fn handle(&mut self, player_id: Id, message: ClientMessage) {
        match message {
            ClientMessage::Action(action) => {
                if let Some(player) = self.players.get_mut(&player_id) {
                    player.action = action;
                }
            }
            ClientMessage::Spawn => {
                if !self.players.contains_key(&player_id) {
                    self.players.insert(
                        player_id,
                        Player::new(
                            player_id,
                            vec2(
                                global_rng().gen_range(0.0, self.rules.world_size),
                                global_rng().gen_range(0.0, self.rules.world_size),
                            ),
                        ),
                    );
                }
            }
        }
    }
}

impl Default for Model {
    fn default() -> Self {
        Self {
            rules: default(),
            current_time: 0.0,
            players: HashMap::new(),
            projectiles: HashMap::new(),
            food: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServerMessage {
    pub model: Model,
    pub client_player_id: Id,
}

impl net::Message for ServerMessage {}

#[derive(Serialize, Deserialize)]
pub enum ClientMessage {
    Action(Action),
    Spawn,
}

impl net::Message for ClientMessage {}
