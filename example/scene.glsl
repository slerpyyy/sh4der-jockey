#version 140

out vec4 color;

uniform vec3 R;

void main() {
    vec2 p = gl_FragCoord.xy / R.xy;
    float t = R.z;

    for(int i=0; i<12; i++) {
        float cut = 0.5 + 0.2 * sin(t);

        if(p.x < cut) {
            p.x /= cut;
			t = 1.1 + t * 1.1;
        } else {
            p.x -= cut;
            p.x /= 1.0 - cut;
            t += 0.5;
		}

        p = p.yx;
    }

    float c = 0.5 * acos(-1.) * t;
    c = 0.5 + 0.5 * cos(c);

    color = vec4(c);
}
