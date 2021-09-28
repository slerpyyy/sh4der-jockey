#version 430

#if 0
	// Blossom header
	layout(location = 0) uniform vec4 iResolution;
	layout(binding = 0) uniform sampler2D accumulatorTex;
#else
	// Sh4derJockey header
	uniform sampler2D accumulatorTex;
	uniform vec4 resolution;
	#define iResolution resolution
#endif

/////////////////////////////////
/// Your code goes below here ///
///////////// V V V /////////////

void main() {
	ivec2 uv = ivec2(gl_FragCoord.xy);

	const float f = 0.2;
	vec4 acc = (1 + 4 * f) * texelFetch(accumulatorTex, uv, 0);
	acc -= f * texelFetch(accumulatorTex, uv + ivec2(0, 1), 0);
	acc -= f * texelFetch(accumulatorTex, uv + ivec2(1, 0), 0);
	acc -= f * texelFetch(accumulatorTex, uv - ivec2(0, 1), 0);
	acc -= f * texelFetch(accumulatorTex, uv - ivec2(1, 0), 0);

	gl_FragColor = vec4(sqrt(acc.rgb / acc.a), 1);
}
