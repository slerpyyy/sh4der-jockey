#version 140

out vec4 color;

uniform vec3 R;
uniform sampler2D B;

vec3 gay(float x) {
    x = x * 3.0 - 1.5;
    return clamp(vec3(-x, 1.0-abs(x), x), 0.0, 1.0);
}

void main() {
    vec2 uv = gl_FragCoord.xy / R.xy;

    vec4 acc = vec4(0);
    const int iter = 40;

    for(int i=0; i<iter; i++) {
        float x = float(i) / float(iter-1);
        float s = 1.0 + 0.1 * (x - 0.5);
        vec2 tuv = 0.5 + s * (uv - 0.5);
        acc += vec4(gay(x), 1) * texture2D(B, tuv);
    }

    acc /= float(iter);

    color = pow(acc, vec4(1) / 2.2);
}
