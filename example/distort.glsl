#version 140

out vec4 color;

uniform vec3 R;
uniform sampler2D tex;

void main() {
    vec2 uv = gl_FragCoord.xy / R.xy;

    uv.x += 0.08 * textureLod(tex, uv, 3.0).x;

    color = texture2D(tex, uv);
}
