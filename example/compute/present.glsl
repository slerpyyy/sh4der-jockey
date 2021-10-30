#version 430

out vec4 color;

uniform vec4 resolution;
uniform sampler2D img_output;

void main() {
    vec2 uv = gl_FragCoord.xy / resolution.xy;
    color = texture(img_output, uv);
}
