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
        let penetration = penetration.min(partial_min(a.size, b.size));
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
        let penetration = penetration.min(partial_min(self.size, target.size));
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
    pub last_hit: Option<Id>,
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
    pub const MAX_AIMING_SPEED: f32 = 4.0;
    pub const ACCELERATION: f32 = 15.0;
    pub const PROJECTILE_MASS_GAIN_SPEED: f32 = 0.3;
    pub const PROJECTILE_COST_SPEED: f32 = 0.1;
    pub const DEATH_SPEED: f32 = 1.0 / 60.0;
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
            last_hit: None,
        }
    }
    fn update(&mut self, delta_time: f32, rules: &Rules) -> Option<Projectile> {
        self.add_mass(-Self::DEATH_SPEED * delta_time);

        let mut target_vel = self.action.target_vel.clamp(1.0) * Self::MAX_SPEED;
        if self.action.shoot {
            target_vel = target_vel.clamp(Self::MAX_AIMING_SPEED);
        }
        self.entity.vel += (target_vel - self.entity.vel).clamp(Self::ACCELERATION * delta_time);
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
            projectile.entity.vel =
                dr * Projectile::UNIT_SIZE_VELOCITY * projectile.entity.size.powf(-0.5);
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
    const UNIT_SIZE_VELOCITY: f32 = 20.0;
    const DEATH_SPEED: f32 = 0.1;
    const STRENGTH: f32 = 2.0;
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
pub enum FoodEvent {
    Add(Food),
    Remove(Id),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Scores {
    pub kills: usize,
    pub deaths: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Event {
    Food(FoodEvent),
    PlayerName { player_id: Id, name: String },
    ScoresUpdate(HashMap<Id, Scores>),
}

pub struct Model {
    pub rules: Rules,
    pub current_time: f32,
    pub players: HashMap<Id, Player>,
    pub projectiles: HashMap<Id, Projectile>,
    pub food: Vec<Food>,
    pub events: Events<Event>,
    scores: HashMap<Id, Scores>,
    player_names: HashMap<Id, String>,
    bots: Vec<Id>,
}

impl Model {
    pub const TICKS_PER_SECOND: f64 = 60.0;
    pub const MAX_FOOD_EXTRA: f32 = 10.0;

    fn add_bot(&mut self) {
        let id = self.new_player();
        self.set_player_name(id, format!("Bot#{}", id.0));
        self.bots.push(id);
    }

    fn spawn(&mut self, id: Id) {
        self.players.insert(
            id,
            Player::new(
                id,
                vec2(
                    global_rng().gen_range(0.0, self.rules.world_size),
                    global_rng().gen_range(0.0, self.rules.world_size),
                ),
            ),
        );
    }

    fn set_player_name(&mut self, id: Id, name: String) {
        self.player_names.insert(id, name.clone());
        self.events.fire(Event::PlayerName {
            player_id: id,
            name,
        });
    }

    pub fn new_player(&mut self) -> Id {
        let id = Id::new();
        self.scores.insert(
            id,
            Scores {
                kills: 0,
                deaths: 0,
            },
        );
        self.scores_updated();
        id
    }
    pub fn disconnect(&mut self, id: Id) {
        self.scores.remove(&id);
        self.players.remove(&id);
        self.player_names.remove(&id);
        self.scores_updated();
    }
    fn scores_updated(&mut self) {
        self.events.fire(Event::ScoresUpdate(self.scores.clone()));
    }
    pub fn tick(&mut self) {
        self.update(1.0 / Self::TICKS_PER_SECOND as f32);
    }
    fn update(&mut self, delta_time: f32) {
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
                    if projectile.hit(player, Projectile::STRENGTH, rules) {
                        player.last_hit = Some(projectile.owner_id);
                    }
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
                let food = Food::new(
                    rules.normalize_pos(
                        pos + vec2(
                            global_rng().gen_range(-1.0, 1.0),
                            global_rng().gen_range(-1.0, 1.0),
                        ) / 5.0,
                    ),
                    rules,
                );
                self.events.fire(Event::Food(FoodEvent::Add(food.clone())));
                self.food.push(food);
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

        let mut scores_updated = false;
        for player in self.players.values() {
            if !player.alive() {
                if let Some(scores) = self.scores.get_mut(&player.id) {
                    scores.deaths += 1;
                }
                if let Some(killer) = player.last_hit {
                    if let Some(scores) = self.scores.get_mut(&killer) {
                        scores.kills += 1;
                    }
                }
                scores_updated = true;
            }
        }

        self.players.retain(|_, e| e.alive());
        self.projectiles.retain(|_, e| e.alive());
        let events = &mut self.events;
        self.food.retain(|e| {
            if e.alive() {
                true
            } else {
                events.fire(Event::Food(FoodEvent::Remove(e.id)));
                false
            }
        });
        if scores_updated {
            self.scores_updated();
        }

        let player_count = self.scores.len() - self.bots.len();
        for i in 0..self.bots.len() {
            let id = self.bots[i];
            if player_count <= 1 && !self.players.contains_key(&id) {
                self.spawn(id);
            }
            if self.players.contains_key(&id) {
                let action = self.think_bot(id);
                self.players.get_mut(&id).unwrap().action = action;
            }
        }
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
                    self.spawn(player_id);
                }
            }
            ClientMessage::SetName(name) => {
                self.set_player_name(player_id, name);
            }
        }
    }
    fn think_bot(&self, id: Id) -> Action {
        let me: &Player = self
            .players
            .values()
            .find(|player| player.id == id)
            .unwrap();
        let closest_food = self.food.iter().min_by(|a, b| {
            self.rules
                .normalize_delta(a.pos - me.pos)
                .len()
                .partial_cmp(&self.rules.normalize_delta(b.pos - me.pos).len())
                .unwrap()
        });
        let closest_enemy = self
            .players
            .values()
            .filter(|player| player.id != me.id)
            .min_by(|a, b| {
                self.rules
                    .normalize_delta(a.pos - me.pos)
                    .len()
                    .partial_cmp(&self.rules.normalize_delta(b.pos - me.pos).len())
                    .unwrap()
            });
        let mut shoot = None;
        if let Some(e) = closest_enemy {
            if self.rules.normalize_delta(e.pos - me.pos).len() < 17.0 {
                shoot = Some(e.pos);
            }
        }
        if me.size < 0.7 {
            shoot = None;
        }
        if let Some(p) = &me.projectile {
            if p.size > 0.4 {
                shoot = None;
            }
        }
        Action {
            target_vel: closest_food.map(|f| f.pos).unwrap_or(vec2(0.0, 0.0)) - me.pos,
            shoot: shoot.is_some(),
            aim: shoot
                .or(me.projectile.as_ref().map(|p| p.pos))
                .unwrap_or(vec2(0.0, 0.0)),
        }
    }
}

impl Default for Model {
    fn default() -> Self {
        let mut result = Self {
            rules: default(),
            current_time: 0.0,
            players: HashMap::new(),
            projectiles: HashMap::new(),
            food: Vec::new(),
            events: Events::new(),
            player_names: HashMap::new(),
            scores: HashMap::new(),
            bots: Vec::new(),
        };
        for _ in 0..5 {
            result.add_bot();
        }
        result
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ModelMessage {
    pub rules: Rules,
    pub current_time: f32,
    pub players: HashMap<Id, Player>,
    pub projectiles: HashMap<Id, Projectile>,
}

impl Model {
    pub fn to_message(&self) -> ModelMessage {
        ModelMessage {
            rules: self.rules.clone(),
            current_time: self.current_time,
            players: self.players.clone(),
            projectiles: self.projectiles.clone(),
        }
    }
    pub fn initial_events(&self) -> Vec<Event> {
        let mut result = Vec::new();
        for food in &self.food {
            result.push(Event::Food(FoodEvent::Add(food.clone())));
        }
        for (id, name) in &self.player_names {
            result.push(Event::PlayerName {
                player_id: *id,
                name: name.clone(),
            });
        }
        result.push(Event::ScoresUpdate(self.scores.clone()));
        result
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServerMessage {
    pub model: ModelMessage,
    pub events: Vec<Event>,
    pub client_player_id: Id,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    Action(Action),
    Spawn,
    SetName(String),
}
