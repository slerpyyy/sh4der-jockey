#version 430

in vec4 v_model_pos;

out vec4 out_color;

uniform vec4 material_base_color;
uniform vec4 resolution;
uniform float time;

const float PI = acos(-1.0);

void main() {
    out_color = vec4( 0.5 + v_model_pos.rgb, 1.0 );
}
