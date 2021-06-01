#version 140

out vec4 v_color;

uniform int vertexCount;
uniform vec4 resolution;
uniform float time;
uniform float beat;

const float pi = acos(-1.0);
const float tau = 2 * pi;

mat2 rot(float a) {
    float s=sin(a), c=cos(a);
    return mat2(c,s,-s,c);
}

vec3 gay(float x) {
    x = x * 3.0 - 1.5;
    return clamp(vec3(-x, 1.0-abs(x), x), 0.0, 1.0);
}

//     6 ---------- 7
//    /|           /|
//   / |          / |
//  4 ---------- 5  |
//  |  |         |  |
//  |  |         |  |
//  |  2 --------|- 3
//  | /          | /
//  |/           |/
//  0 ---------- 1

vec3 side(int id) { // 6v
    id = (id & 3) + 1;
    float f = 2 * (id >> 2) - 1;
    vec2 p = ivec2(id, id >> 1) & 1;
    return vec3(2 * f * p + 1, 1);
}

vec3 cube(int id) { // 36v
    vec3 p = side(id); id /= 6;
    float f = 2 * (id & 1) - 1;
    id = (id >> 1) % 3;
    while (id --> 0) p = p.zxy;
    return f * p;
}

void main() {
    vec3 p = cube(gl_VertexID);

    float t = 36 * float(gl_VertexID / 36) / vertexCount;
    float a = time + 8 * tau * t;

    p *= 0.5 * pow(1 - fract(beat), 5);
    p += cos(vec3(1.2, 1.1, 1.3) * a);
    p.xz *= rot(a);
    p.xy *= rot(2 * a);

    p.xz *= rot(time);
    p.yz *= rot(0.3);
    p.z += 2;
    p.x *= resolution.w;
    p /= p.z + 1;

    gl_Position = vec4(p, 1);
    gl_PointSize = 1.0;

    float temp = 0.5 + 0.5 * sin(0.01 * gl_VertexID);
    v_color = vec4(pow(gay(temp), vec3(1) / 2.2), 1);
}
