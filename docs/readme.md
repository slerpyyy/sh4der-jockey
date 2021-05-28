# Sh4der Jockey
*A custom VJ tool written by sp4ghet and slerpy*

## Getting started

First put the executable in a place where it is allowed to create new files. It currently generates only a single file to store the window layout of the control panel, but this might change in the future.

Then run the tool in your project folder with the `-i` flag. This will instruct the tool to set up a simple example project.

## Pipeline

Once the tools is starts, it looks for files ending in `.yaml` in the current working directory and treats these as pipeline files.

...

## Fragment Shaders

```glsl
#version 140

out vec4 out_color;

uniform vec4 resolution;
uniform float time;

void main() {
    vec2 uv = gl_FragCoord.xy / resolution.xy;
    out_color = vec4(uv, 0.5 + 0.5 * sin(time), 1);
}
```

### Required fields

A fragment shader stage must contain the following fields

 - `fs: Path` Specifies the file name of the fragment shader file.

### Optional field

 - `target: String` Specifies the name of the render target.
 - `resolution: [Integer; 2]` Sets the size of the target framebuffer.
 - `wrap: {clamp, repeat}` Sets the wrapping mode of the target.
 - `filter: {linear, nearest}` Sets the wrapping mode of the target.
 - `mipmap: Bool` Enables or disables mipmapping for the target.
 - `float: Bool` Changes the way data is stored in the target.


## Vertex Shaders
## Compute Shaders

## Images

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
uniform float beat;
```
A timer, which increases by one each beat. This uniform is controlled by the _Beat Sync_ window in the control panel. To get a scaled time since the last beat, use `fract(beat)`.

```glsl
uniform float sliders[32];
```
The position of each slider. This uniform is controlled by the _Sliders_ window in the control panel.

```glsl
uniform vec4 buttons[32];
```
??? how the hell do i phrase this

```glsl
uniform int vertexCount;
```
The total number of vertices used. This uniform only applies to vertex shader stages.

```glsl
out vec4 out_color;
```
The final color of the pixel. This may only be written to by the fragment shader. If a stage does not contain a user defined fragment shader, the value written to `v_color` will be forwarded.

```glsl
in vec2 position;
```
I don't even know what this is for...

```glsl
uniform sampler1D samples;
```
The raw samples taken from the default audio in.

```glsl
uniform sampler1D raw_spectrum;
```

```glsl
uniform sampler1D spectrum;
```

```glsl
uniform vec3 volume;
```

```glsl
uniform sampler3D noise;
```
A 32 x 32 x 32 noise texture. Please note that this texture is generated when the pipeline is build, so the noise pattern will look different every time the pipeline is reloaded.
