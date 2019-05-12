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
    entity: Entity,
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
    pub fn new() -> Self {
        Self {
            entity: Entity {
                pos: vec2(0.0, 0.0),
                vel: vec2(0.0, 0.0),
                size: 1.0,
            },
            action: default(),
        }
    }
    fn update(&mut self, delta_time: f32) {
        self.entity.vel += (self.action.target_vel.clamp(1.0) * Self::MAX_SPEED - self.entity.vel)
            .clamp(Self::ACCELERATION * delta_time);
        self.entity.update(delta_time);
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Model {
    pub players: HashMap<PlayerId, Player>,
}

impl Model {
    pub fn update(&mut self, delta_time: f32) {
        for player in self.players.values_mut() {
            player.update(delta_time);
        }
    }
}

impl Default for Model {
    fn default() -> Self {
        Self {
            players: HashMap::new(),
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
