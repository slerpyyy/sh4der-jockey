#version 140

out vec4 color;

uniform vec3 R;
uniform sampler2D tex;

void main() {
    vec2 uv = gl_FragCoord.xy / R.xy;

    uv.x += 0.03 * textureLod(tex, uv, 3.2).x;

    color = texture2D(tex, uv);
}
