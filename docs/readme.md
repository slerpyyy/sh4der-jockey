# Sh4der Jockey
*A custom VJ tool written by sp4ghet and slerpy*

## Uniforms

```glsl
uniform vec4 resolution;
```
The resolution of the render target.
The `z` and `w` components hold the aspect ratio in the corresponding direction,
so `z == x / y` and `w == y / x`.

```glsl
uniform int passIndex;
```
The index of the current stage in the pipeline.
This is (semi) useful for reusing shaders multiple times within the same pipeline.

```glsl
uniform float time;
```
The time in seconds since the tool has been started.

```glsl
uniform float delta;
```
The time in seconds elapsed since the last frame.

```glsl
beat
```

```glsl
sliders
```

```glsl
buttons
```

```glsl
vertexCount
```

```glsl
out_color
```

```glsl
position
```

```glsl
samples
```

```glsl
raw_spectrum
```

```glsl
spectrum
```

```glsl
noise
```

```glsl
volume
```

## Fragment Shaders

```glsl
#version 140

out vec4 color;

uniform vec3 resolution;
uniform float time;

void main() {
    vec2 uv = gl_FragCoord.xy / R.xy;
    color = vec4(uv, 0.5 + 0.5 * sin(time), 1);
}
```

### Required fields

A fragment shader stage must contain the following fields

 - `fs`: Specifies the file name of the fragment shader file.

### Optional field

 - `target`: Specifies the name of the render target.
 - ...


## Vertex Shaders
## Compute Shaders
