#version 140

out vec4 color;

uniform vec3 R;
uniform sampler2D tex;
uniform float buttons[8];

void main() {
    vec2 uv = gl_FragCoord.xy / R.xy;

    uv.x += (0.5 * exp(-8*buttons[0]) + 0.03) * (textureLod(tex, uv, 3.2).x - 0.5);

    color = texture2D(tex, uv);
}
