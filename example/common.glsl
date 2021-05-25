uniform vec3 R;

uniform sampler2D tex;
uniform sampler2D distorted;

uniform float sliders[32];
uniform vec4 buttons[32];

vec3 gay(float x) {
    x = x * 3.0 - 1.5;
    return clamp(vec3(-x, 1.0-abs(x), x), 0.0, 1.0);
}
