#version 430

#if 0
	// Blossom header
	layout(location = 0) uniform vec4 iResolution;
	layout(location = 1) uniform int iFrame;
#else
	// Sh4derJockey header
	uniform vec4 resolution;
	uniform int frame_count;
	#define iResolution resolution
	#define iFrame frame_count
#endif

/////////////////////////////////
/// Your code goes below here ///
///////////// V V V /////////////

const float pi = acos(-1.0);

vec4 rv;
void shuffle() {
	rv = fract(sin(1e4*rv) + rv.wxyz);
}

// adapted from 0b5vr
// https://www.shadertoy.com/view/fdVGDy
vec3 noise(vec3 p, float f) {
	vec4 acc = vec4(0);
	for (int i=0; i<6; i++) {
		p += p; p += sin(p.yzx);
		acc = f * acc + vec4(cross(sin(p.zxy), cos(p)), 1.0);
	}

	return acc.xyz / acc.w;
}

mat2 rot(float a) {
	float s=sin(a), c=cos(a);
	return mat2(c,-s,s,c);
}

float logo(vec2 p) {
	const float t = 0.05;
    const float h = 0.5 * (7.0 / 18.0 + t);

    p *= sign(p.x);
    return abs(min(
        max(p.x, abs(p.y+h)+h)-1,
        max(sqrt(.5)*(abs(p.x-p.y+1)-1), p.y-1)
    )) - t;
}


float trace(vec3 ro, vec3 rd) {
	float r = 1e32;

	for(int i = -8; i <= 2; i++) {
		float t = (i - ro.y) / rd.y;
		if (t < 0) continue;

		vec3 p = ro + rd * t;
		p.xz *= rot(0.06 * p.y - 0.5);

		float d = logo(p.xz);

		vec2 a = sqrt(p.xz * p.xz + 0.3) - 1.5;
		d = min(d, noise(p, 0.9).x + 0.05 * (p.y - p.z + 9) - min(0, max(a.x, a.y)));

		a = abs(a * rot(0.2) - 2);
		a = abs(a * rot(-0.1) - 1);

		d = min(d, min(a.x, a.y));

		if (d < 0.01 * (rv.x - 0.5)) r = min(r, t);
		rv = rv.wxyz;
	}

	return r;
}

void main() {
	rv = 7 * cos(iFrame + gl_FragCoord);
	for (int i=0; i<8; i++) shuffle();

	vec2 aa = rv.xy * 2 - 1;
	vec2 uv = (2 * gl_FragCoord.xy - iResolution.xy + aa) / iResolution.y;

	vec2 a = vec2(1, 2 * pi) * rv.zw;
    vec2 dof = 0.5 * pow(a.x, 0.45) * vec2(cos(a.y),sin(a.y));

	const float focal = 8.0;
	vec3 ro = vec3(dof, -3.5 * focal);
    vec3 rd = normalize(vec3(uv - focal * dof / length(ro), focal));

	ro.yz *= rot(0.7);
	rd.yz *= rot(0.7);

	shuffle();
    shuffle();
    shuffle();

	float t = trace(ro, rd);
	vec3 col = (1 + 2 * vec3(aa, aa.x)) * exp(0.5 * (length(ro) - t - 1));
	gl_FragColor = vec4(col, 1);
}
