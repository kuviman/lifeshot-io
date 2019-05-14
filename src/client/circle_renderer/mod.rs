use crate::*;

#[derive(ugli::Vertex)]
struct QuadVertex {
    a_pos: Vec2<f32>,
}

#[derive(ugli::Vertex, Debug)]
pub struct Instance {
    pub i_pos: Vec2<f32>,
    pub i_size: f32,
    pub i_color: Color<f32>,
}

pub struct CircleRenderer {
    quad_geometry: ugli::VertexBuffer<QuadVertex>,
    instances: ugli::VertexBuffer<Instance>,
    program: ugli::Program,
}

impl CircleRenderer {
    pub fn new(context: &Rc<geng::Context>) -> Self {
        Self {
            quad_geometry: ugli::VertexBuffer::new_static(
                context.ugli_context(),
                vec![
                    QuadVertex {
                        a_pos: vec2(-1.0, -1.0),
                    },
                    QuadVertex {
                        a_pos: vec2(1.0, -1.0),
                    },
                    QuadVertex {
                        a_pos: vec2(1.0, 1.0),
                    },
                    QuadVertex {
                        a_pos: vec2(-1.0, 1.0),
                    },
                ],
            ),
            instances: ugli::VertexBuffer::new_dynamic(context.ugli_context(), Vec::new()),
            program: context
                .shader_lib()
                .compile(include_str!("program.glsl"))
                .unwrap(),
        }
    }
    pub fn queue(&mut self, instance: Instance) {
        self.instances.push(instance);
    }
    pub fn draw(
        &mut self,
        framebuffer: &mut ugli::Framebuffer,
        view_matrix: Mat4<f32>,
        rules: &Rules,
    ) {
        for i in -1..=1 {
            for j in -1..=1 {
                ugli::draw(
                    framebuffer,
                    &self.program,
                    ugli::DrawMode::TriangleFan,
                    ugli::instanced(&self.quad_geometry, &self.instances),
                    ugli::uniforms! {
                        u_view_matrix: view_matrix,
                        u_world_offset: vec2(i as f32 * rules.world_size, j as f32 * rules.world_size),
                    },
                    ugli::DrawParameters {
                        blend_mode: Some(default()),
                        ..default()
                    },
                );
            }
        }
        self.instances.clear();
    }
}
