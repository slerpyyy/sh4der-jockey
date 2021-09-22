#version 140

out vec4 out_color;

uniform vec4 resolution;
uniform float time;
uniform float time_delta;

const float width = 0.01;
const float size = 0.25;
const int index = 4;

const float fade = 2.5;
const float bias = 0.5;
const float flicker = 2.0;

const float aa = 0.9;
const float blur = 1.2;
const int samples = 4;

// credit to yx
// https://www.shadertoy.com/view/tdfcWX
float bayer(ivec2 p) {
    int s = 0;
    p.y ^= p.x;
    s += ((p.x >> 2) & 1) << 0;
    s += ((p.y >> 2) & 1) << 1;
    s += ((p.x >> 1) & 1) << 2;
    s += ((p.y >> 1) & 1) << 3;
    s += ((p.x >> 0) & 1) << 4;
    s += ((p.y >> 0) & 1) << 5;
    return s / 64.0;
}

void main() {
    vec2 uv = (2 * gl_FragCoord.xy - resolution.xy) / resolution.y;
    float b = bayer(ivec2(gl_FragCoord));
    float r = mix(resolution.z, 16.0 / 9.0, smoothstep(0.0, 2.0, time));

    out_color = vec4(0);

    for (int s = 0; s < samples; s++) {
        vec4 acc = vec4(0);
        float tt = time + blur * time_delta * (s + b) / samples;

        if (time < 2.0) {
            for (int i = 1; i < 32; i++) {
                float t = smoothstep(0, 1, tt / 2.0);
                float l = (t * i + bias) / (index + bias);
                vec2 a = abs(uv) - size * vec2(r, 1) / l;
                float d = abs(max(a.x, a.y)) - width / l;
                if (d > aa) continue;

                float col = min(exp(fade * (1 - l)), 1.0) * smoothstep(-aa, aa, -d * resolution.y);
                if (i != index) col *= pow(1 - fract(12 * max(time, 1.6667)), flicker);
                acc = max(acc, col);
            }
        } else {
            vec2 p = uv * (step(0, uv.x - uv.y) * 2 - 1);

            float t = 5 * (tt - 2);
            t = smoothstep(0, 1, t / (t + 1)) * size * (r - 1) / 2;
            p += t;

            vec2 a = abs(p) - size * vec2(r, 1);
            float d = abs(max(a.x, a.y)) - width;
            d = min(d, max((p.x - p.y - 3 * width) / sqrt(2), max(size - p.y, p.y - size - 2 * t - width)));

            acc = vec4(smoothstep(-aa, aa, -d * resolution.y));
        }

        out_color += acc;
    }

    out_color /= samples;
}
