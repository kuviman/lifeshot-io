use crate::*;

#[derive(ugli::Vertex)]
struct QuadVertex {
    a_pos: Vec2<f32>,
}

pub struct TextureRenderer {
    quad_geometry: ugli::VertexBuffer<QuadVertex>,
    program: ugli::Program,
}

impl TextureRenderer {
    pub fn new(geng: &Rc<Geng>) -> Self {
        Self {
            quad_geometry: ugli::VertexBuffer::new_static(
                geng.ugli(),
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
            program: geng
                .shader_lib()
                .compile(include_str!("program.glsl"))
                .unwrap(),
        }
    }
    pub fn draw(
        &mut self,
        framebuffer: &mut ugli::Framebuffer,
        view_matrix: Mat4<f32>,
        pos: Vec2<f32>,
        texture: &ugli::Texture,
        rules: &Rules,
    ) {
        let size = {
            let size = texture.get_size().map(|x| x as f32);
            let height = 0.5;
            vec2(size.x * height / size.y, height)
        };
        for i in -1..=1 {
            for j in -1..=1 {
                ugli::draw(
                    framebuffer,
                    &self.program,
                    ugli::DrawMode::TriangleFan,
                    &self.quad_geometry,
                    ugli::uniforms! {
                        u_view_matrix: view_matrix,
                        u_world_offset: vec2(i as f32 * rules.world_size, j as f32 * rules.world_size),
                        u_pos: pos,
                        u_size: size,
                        u_texture: texture,
                    },
                    ugli::DrawParameters {
                        blend_mode: Some(default()),
                        ..default()
                    },
                );
            }
        }
    }
}
