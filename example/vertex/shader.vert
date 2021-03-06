#version 140

out vec4 v_color;

uniform int vertex_count;
uniform vec4 resolution;
uniform float time;

const float pi = acos(-1.0);

mat2 rot(float a) {
    float s=sin(a), c=cos(a);
    return mat2(c,s,-s,c);
}

void main() {
    float a = 8.0 * pi * gl_VertexID / vertex_count;

    float r = 26.0 / 8.0;
    vec2 off = vec2(sin(r*a) + 2.3, cos(r*a) + 1.0);
    vec3 p = vec3(sin(a), 1, cos(a)) * off.xyx;

    p.xz *= rot(time);
    p.yz *= rot(0.6);

    v_color = vec4(1.0 / abs(p.z + 1.0));

    p.z += 5;
    p.x *= resolution.w;

    gl_Position = vec4(p, p.z);
}
