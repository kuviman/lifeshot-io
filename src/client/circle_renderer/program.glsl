varying vec2 v_pos;
varying vec4 v_color;

#ifdef VERTEX_SHADER
attribute vec2 a_pos;
attribute vec2 i_pos;
attribute float i_size;
attribute vec4 i_color;
uniform mat4 u_view_matrix;
uniform vec2 u_world_offset;
void main() {
    v_pos = a_pos;
    v_color = i_color;
    gl_Position = u_view_matrix * vec4(u_world_offset + i_pos + a_pos * i_size, 0.0, 1.0);
}
#endif

#ifdef FRAGMENT_SHADER
void main() {
    if (length(v_pos) > 1.0) {
        discard;
    }
    gl_FragColor = v_color;
}
#endif