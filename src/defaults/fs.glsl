#version 140

out vec4 color;

uniform vec3 R;

void main() {
    vec2 uv = gl_FragCoord.xy / R.xy;
    color = vec4(uv, 0.5 + 0.5 * sin(R.z), 1);
}
