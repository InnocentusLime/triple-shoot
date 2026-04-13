in vec2 pos;
in vec2 uv;

out vec2 texcoord;

uniform mat4 view_projection;

void main() {
    gl_Position = view_projection * vec4(pos, 0, 1);
    texcoord = uv;
}