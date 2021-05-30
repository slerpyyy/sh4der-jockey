#version 440

out vec4 out_color;

uniform sampler1D samples;
uniform vec4 resolution;
uniform sampler2D osci;
uniform sampler2D ring;

float graph(float y, float f, float t) {

    return smoothstep(f + t, f, y) * smoothstep(f - t, f, y);
}

void main() {
    vec2 uv = gl_FragCoord.xy / resolution.xy;
    vec3 samp = texture(samples, uv.x).rgb + .5;

    vec3 c = vec3(0.01);
    c += texture(osci, uv).rgb;
    c += texture(ring, uv).rgb;

    c = pow(c, vec3(.4545));

    out_color = vec4(c, 1.);
}
