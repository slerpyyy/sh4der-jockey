#version 430

out vec4 color;

uniform vec3 R;
uniform float time;
uniform sampler2D img_output;

void main() {
    vec2 p = gl_FragCoord.xy / R.xy;

    color = texture2D(img_output, p);
}
