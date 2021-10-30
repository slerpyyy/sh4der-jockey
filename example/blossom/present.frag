#version 430

// Once this file has been moved to the framework
// you can remove everything but the Blossom header
// from this section in both shaders

#ifdef SH4DERJOCKEY

    // compatibility header
    out vec4 fragColor;
    uniform sampler2D accumulatorTex;
    uniform vec4 resolution;
    #define iResolution resolution
    #define gl_FragColor fragColor

#else

    // Blossom header
    layout(location = 0) uniform vec4 iResolution;
    layout(binding = 0) uniform sampler2D accumulatorTex;

#endif

/////////////////////////////////
/// Your code goes below here ///
///////////// V V V /////////////

void main() {
    ivec2 coord = ivec2(gl_FragCoord.xy);

    const float f = 0.2;
    vec4 acc = (1 + 4 * f) * texelFetch(accumulatorTex, coord, 0);
    acc -= f * texelFetch(accumulatorTex, coord + ivec2(0, 1), 0);
    acc -= f * texelFetch(accumulatorTex, coord + ivec2(1, 0), 0);
    acc -= f * texelFetch(accumulatorTex, coord - ivec2(0, 1), 0);
    acc -= f * texelFetch(accumulatorTex, coord - ivec2(1, 0), 0);

    gl_FragColor = vec4(sqrt(acc.rgb / acc.a), 1);
}
