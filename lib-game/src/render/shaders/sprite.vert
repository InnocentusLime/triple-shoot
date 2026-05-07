uniform mat4 view_projection;
uniform vec2 width_height;

layout(location=0) in vec2 v_pos;
layout(location=1) in vec2 v_uv_not_normalized;
layout(location=2) in vec4 v_color;

out vec2 f_uv;
out vec4 f_color;

void main() {
    vec2 uv = v_uv_not_normalized / width_height;
    uv.y = 1.0 - uv.y;
    
    f_uv = uv;
    f_color = v_color;
    
    gl_Position = view_projection * vec4(v_pos, 0, 1);
}