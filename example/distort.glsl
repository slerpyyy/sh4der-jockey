#version 140

out vec4 color;

#pragma include "common.glsl"

void main() {
    vec2 uv = gl_FragCoord.xy / R.xy;

    uv.x += (0.5 * exp(-8*buttons[0].y) + 0.03) * (textureLod(tex, uv, 3.2).x - 0.5);

    color = texture2D(tex, uv);
}
