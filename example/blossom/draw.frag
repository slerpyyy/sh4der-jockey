#version 430

// Once this file has been moved to the framework
// you can remove everything but the Blossom header
// from this section in both shaders

#ifdef SH4DERJOCKEY

    // compatibility header
    out vec4 fragColor;
    uniform vec4 resolution;
    uniform int frame_count;
    #define iResolution resolution
    #define iFrame frame_count

#else

    // Blossom header
    layout(location = 0) out vec4 fragColor;
    layout(location = 0) uniform vec4 iResolution;
    layout(location = 1) uniform int iFrame;

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

// rotation matrix
mat2 rot(float a) {
    float s=sin(a), c=cos(a);
    return mat2(c,-s,s,c);
}

// Sh4derJockey logo sdf
float logo(vec2 p) {
    const float t = 0.015;
    const float h = 0.5 * (7.0 / 18.0 + t);

    p *= sign(p.x);
    return abs(min(
        max(p.x, abs(p.y+h)+h)-1,
        max(sqrt(.5)*(abs(p.x-p.y+1)-1), p.y-1)
    )) - t;
}


float trace(vec3 ro, vec3 rd) {
    float r = 1e32;

    // intersect with one slice at a time
    for(int i = -8; i <= 2; i++) {

        // ray-plane intersection
        float t = (i - ro.y) / rd.y;
        if (t < 0) continue;

        // compute point on plane
        vec3 p = ro + rd * t;
        float s = dot(p, rd);
        p.xz *= rot(0.06 * p.y - 0.5);

        // evaluate logo sdf
        float d = logo(p.xz);

        // add noise pattern
        if (i < 1) {
            d = min(d, abs(noise(p, 5).x) - 0.002);

            vec2 a = sqrt(p.xz * p.xz + 0.3) - 1.5;
            d = min(d, noise(p, 0.9).x + 0.05 * (9 - s) - min(0, max(a.x, a.y)));

            a = abs(a * rot(0.2) - 2);
            a = abs(a * rot(-0.1) - 1);

            d = min(d, min(a.x, a.y));
        }

        // update nearest intersection
        if (d < 0.01 * (rv.x - 0.5)) r = min(r, t);

        // cycle random state
        rv = rv.wxyz;
    }

    return r;
}

void main() {
    // initialise random state
    rv = 7 * cos(iFrame + gl_FragCoord);
    for (int i=0; i<8; i++) shuffle();

    // compute uv coordinates with anti-aliasing
    vec2 aa = rv.xy * 2 - 1;
    vec2 uv = (2 * gl_FragCoord.xy - iResolution.xy + aa) / iResolution.y;

    // compute depth of field parameters
    vec2 a = vec2(1, 2 * pi) * rv.zw;
    vec2 dof = 0.6 * pow(a.x, 0.45) * vec2(cos(a.y),sin(a.y));

    // setup camera
    const float focal = 8.0;
    vec3 ro = vec3(dof, -3.5 * focal);
    vec3 rd = normalize(vec3(uv - focal * dof / length(ro), focal));

    // orbit rotate camera
    ro.yz *= rot(0.7);
    rd.yz *= rot(0.7);

    // update random state
    shuffle();
    shuffle();
    shuffle();

    // trace and compute color
    float t = trace(ro, rd);
    vec3 col = (1 + 2 * vec3(aa, aa.x)) * exp(-0.1*pow(length(ro) - t - 1, 2));

    // set output color
    fragColor = vec4(col, 1);
}
