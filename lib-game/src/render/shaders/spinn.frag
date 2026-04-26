in vec2 o_texcoord;

out vec4 f_color;

uniform float progress;
uniform sampler2D tex;

void main() {
    vec3 tint = vec3(1.0 - progress, progress, 0.0);
    vec4 col = texture(tex, o_texcoord);
    f_color = vec4(float(col.x <= progress) * tint, col.w);
}