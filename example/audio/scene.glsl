#version 440

out vec4 out_color;

uniform sampler1D samples;
uniform vec4 resolution;

float graph(float y, float f, float t) {

    return smoothstep(f + t, f, y) * smoothstep(f - t, f, y);
}

void main() {
    vec2 uv = gl_FragCoord.xy / resolution.xy;
    float samp = texture(samples, uv.x).r + .5;

    vec3 c = vec3(1.);
    c *= graph(uv.y, samp, 0.05);
    out_color = vec4(c, 1.);
}
