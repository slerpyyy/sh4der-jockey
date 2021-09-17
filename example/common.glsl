#pragma once

uniform vec4 resolution;
uniform float time;

uniform sampler2D tex;

uniform float sliders[32];
uniform vec4 buttons[32];

vec3 rainbow(float x) {
    x = x * 3.0 - 1.5;
    return clamp(vec3(-x, 1.0-abs(x), x), 0.0, 1.0);
}
