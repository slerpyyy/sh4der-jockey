#version 140

out vec4 out_color;

uniform vec4 resolution;
uniform float time;

const float width = 0.01;
const float size = 0.25;
const int index = 4;

const float fade = 2.5;
const float bias = 0.5;
const float flicker = 2.0;
const float aa = 1.1;

void main() {
    vec2 uv = (2 * gl_FragCoord.xy - resolution.xy) / resolution.y;

    vec4 acc = vec4(0);

    float r = mix(resolution.z, 16.0 / 9.0, smoothstep(0.0, 2.0, time));

    if (time < 2.0) {
        for (int i = 1; i < 32; i++) {
            float t = smoothstep(0, 1, time / 2.0);
            float l = (t * i + bias) / (index + bias);
            vec2 a = abs(uv) - size * vec2(r, 1) / l;
            float d = abs(max(a.x, a.y)) - width / l;

            float col = min(exp(fade * (1 - l)), 1.0) * smoothstep(-aa, aa, -d * resolution.y);
            if (i != index) col *= pow(1 - fract(12 * max(time, 1.667)), flicker);
            acc = max(acc, col);
        }
    } else {
        uv *= step(0, uv.x - uv.y) * 2 - 1;

        float t = 5 * (time - 2);
        t = smoothstep(0, 1, t / (t + 1)) * size * (r - 1) / 2;
        uv += t;

        vec2 a = abs(uv) - size * vec2(r, 1);
        float d = abs(max(a.x, a.y)) - width;
        d = min(d, max((uv.x - uv.y - 3 * width) / sqrt(2), max(size - uv.y, uv.y - size - 2 * t - width)));

        float col = smoothstep(-aa, aa, -d * resolution.y);
        acc = max(acc, col);
    }

    out_color = acc;
}
