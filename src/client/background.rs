use super::*;

struct Particle {
    pos: Vec2<f32>,
    size: f32,
    vel: Vec2<f32>,
    color: Color<f32>,
}

pub struct Background {
    particles: Vec<Particle>,
    rules: Rules,
}

impl Background {
    pub fn new(rules: &Rules) -> Self {
        let mut particles = Vec::new();
        for _ in 0..10 {
            particles.push(Particle {
                pos: vec2(
                    global_rng().gen_range(0.0, rules.world_size),
                    global_rng().gen_range(0.0, rules.world_size),
                ),
                size: global_rng().gen_range(ClientApp::CAMERA_FOV / 2.0, ClientApp::CAMERA_FOV),
                color: Color::rgba(
                    global_rng().gen_range(0.0, 1.0),
                    global_rng().gen_range(0.0, 1.0),
                    global_rng().gen_range(0.0, 1.0),
                    0.02,
                ),
                vel: Vec2::rotated(
                    vec2(1.0, 0.0),
                    global_rng().gen_range(0.0, 2.0 * std::f32::consts::PI),
                ),
            });
        }
        Self {
            rules: rules.clone(),
            particles,
        }
    }
    pub fn update(&mut self, delta_time: f32) {
        for p in &mut self.particles {
            p.pos = self.rules.normalize_pos(p.pos + p.vel * delta_time);
        }
    }
    pub fn draw(&self, renderer: &mut CircleRenderer) {
        for p in &self.particles {
            renderer.queue(circle_renderer::Instance {
                i_pos: p.pos,
                i_size: p.size,
                i_color: p.color,
            });
        }
    }
}
