#version 140

out vec4 color;

uniform vec3 R;
uniform sampler2D tex;

void main() {
    vec2 uv = gl_FragCoord.xy / R.xy;

    uv.x += 0.1 * sin(sin(103.5 * floor(32.0 * uv.y)) * 8.951 + R.z);

    color = texture2D(tex, uv);
}
