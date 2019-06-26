varying vec2 v_pos;

#ifdef VERTEX_SHADER
attribute vec2 a_pos;
uniform mat4 u_view_matrix;
uniform vec2 u_world_offset;
uniform vec2 u_pos;
uniform vec2 u_size;
void main() {
    v_pos = (a_pos + 1.0) / 2.0;
    gl_Position = u_view_matrix * vec4(u_world_offset + u_pos - u_size / 2.0 + u_size * v_pos, 0.0, 1.0);
}
#endif

#ifdef FRAGMENT_SHADER
uniform sampler2D u_texture;
void main() {
    gl_FragColor = texture2D(u_texture, v_pos);
}
#endif