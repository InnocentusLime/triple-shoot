in vec2 texcoord;

out vec4 f_color;

uniform sampler2D tex;
uniform vec2 res;

vec2 uv_klems( vec2 uv, vec2 texture_size ) {
            
    vec2 pixels = uv * texture_size + 0.5;
    
    // tweak fractional value of the texture coordinate
    vec2 fl = floor(pixels);
    vec2 fr = fract(pixels);
    vec2 aa = fwidth(pixels) * 0.75;

    fr = smoothstep( vec2(0.5) - aa, vec2(0.5) + aa, fr);
    
    return (fl + fr - 0.5) / texture_size;
}

void main() {
    f_color = texture(tex, uv_klems(texcoord, res));
}