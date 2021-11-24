#version 430

// Once this file has been moved to the framework
// you can remove everything but the Blossom header
// from this section in both shaders

#ifdef SH4DERJOCKEY

    // compatibility header
    out vec4 fragColor;
    uniform vec4 resolution;
    uniform sampler2D accumulatorTex;
    #define iResolution resolution

#else

    // Blossom header
    layout(location = 0) out vec4 fragColor;
    layout(location = 0) uniform vec4 iResolution;
    layout(binding = 0) uniform sampler2D accumulatorTex;

#endif

/////////////////////////////////
/// Your code goes below here ///
///////////// V V V /////////////

void main() {
    ivec2 coord = ivec2(gl_FragCoord.xy);

    // apply unsharp filter
    const float f = 0.2;
    vec4 acc = (1 + 4 * f) * texelFetch(accumulatorTex, coord, 0);
    acc -= f * texelFetch(accumulatorTex, coord + ivec2(0, 1), 0);
    acc -= f * texelFetch(accumulatorTex, coord + ivec2(1, 0), 0);
    acc -= f * texelFetch(accumulatorTex, coord - ivec2(0, 1), 0);
    acc -= f * texelFetch(accumulatorTex, coord - ivec2(1, 0), 0);

    // set output color
    fragColor = vec4(sqrt(acc.rgb / acc.a), 1);
}
