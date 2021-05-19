#version 440

out vec4 v_color;

uniform sampler1D samples;
uniform vec4 resolution;
uniform float vertexCount;

const float PI = acos(-1.);
const float TAU = 2. * PI;

vec3 grad(vec3 off, vec3 amp, vec3 fre, vec3 pha, float t) {
    return off * .5 + 0.5 * amp * cos(TAU * (fre * t + pha));
}

mat2 r2d(float t) {
    float c = cos(t), s = sin(t);
    return mat2(c, s, -s, c);
}

void main() {
    int vid = gl_VertexID;
    float x = float(vid) / vertexCount;
    vec2 samp = texture(samples, x).rg;

    vec3 c = grad(vec3(0.8, 0.5, 0.4), vec3(0.2, 0.4, 0.2), vec3(2.0, 1.0, 1.0), vec3(0.0, 0.25, 0.25), x);

    vec3 p = vec3(samp.r, samp.g, 0.);
    p.xy *= .5;
    p.xy *= r2d(-PI * .25);
    p.x *= resolution.w;

    gl_Position = vec4(p, 1);
    gl_PointSize = 4.0;

    v_color = vec4(c, 1.);
}
