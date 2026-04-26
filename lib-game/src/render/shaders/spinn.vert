in vec2 pos;
in vec2 uv;

out vec2 o_texcoord;

uniform mat4 view_projection;

void main() {
    gl_Position = view_projection * vec4(pos, 0, 1);
    vec2 res_uv = uv;
    res_uv.y = 1.0 - res_uv.y;
    o_texcoord = res_uv;
}