# Sh4der Jockey
*A custom VJ tool written by sp4ghet and slerpy*

## Getting started

First put the executable in a place where it is allowed to create new files. It currently generates only a single file to store the window layout of the control panel, but this might change in the future.

Then run the tool in your project folder with the `-i` flag. This will instruct the tool to set up a simple example project.

## Pipeline

Once the tools is starts, it looks for files ending in `.yaml` in the current working directory and treats these as pipeline files.

Below is an example pipeline file.
You can have multiple pipelines in the working directory and choose from the Control Panel.
```yaml
stages:
  - cs: "./particle_pos.comp"
    target: "particle_pos"
    resolution: [10000, 200, 2]
    dispatch_size: [100, 200, 1]
  - vs: "draw_particle.vert"
    count: 8000000
    mode: LINES
    target: particles
    point_size: 2

  - fs: "scenes/scene_a.glsl"
    target: "render"
    float: true
    mipmap: true
  - fs: "./post_process.frag"
images:
  - path: "./images/image.png"
    name: "some_image"
```

## Fragment Shaders

```glsl
#version 440

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

### Optional fields

 - `target: String` Specifies the name of the render target.
   - default: framebuffer for display
 - `resolution: [Int; 2]` Sets the size of the target framebuffer.
   - default: window resolution
 - `wrap: {clamp, repeat}` Sets the wrapping mode of the target.
   - default: clamp
 - `filter: {linear, nearest}` Sets the wrapping mode of the target.
   - default: linear
   - gets set to MIPMAP_X when `mipmap` is `true`
 - `mipmap: Bool` Enables or disables mipmapping for the target.
   - default: false
 - `float: Bool` Changes the way data is stored in the target.
   - default: false


### Unique Uniforms and Varyings

```glsl
// The final color of the pixel. This may only be written to by the fragment shader.
// If a stage does not contain a user defined fragment shader, the value written to `v_color` will be forwarded.
out vec4 out_color;
```

## Vertex Shaders

```glsl
#version 140

out vec4 v_color;
uniform int vertexCount;

void main(){

   v_color = vec4(1);
   gl_VertexPos
}
```

### Required fields

A vertex shader stage must contain the following fields

 - `vs: Path` Specifies the file name of the vertex shader file.

### Optional fields

 - `count: Int` Set the number of vertices to draw.
    - default: 2000
 - `mode: {LINE_LOOP, LINE_STRIP, LINES, POINTS, TRIANGLE_FAN, TRIANGLE_STRIP, TRIANGLES}`
    - default: POINTS
    - maps directly to the respective mode in OpenGL
 - `thickness: Float` The thickness with which to draw points and lines.
    - default: 1
 - `fs: Path` Specifies the file name of the fragment shader file.
    - default: default fragment shader.
 - `target: String` Specifies the name of the render target.
    - default: framebuffer for display
 - `resolution: [Int; 2]` Sets the size of the target framebuffer.
    - default: window resolution
 - `wrap: {clamp, repeat}` Sets the wrapping mode of the target.
    - default: clamp
 - `filter: {linear, nearest}` Sets the wrapping mode of the target.
    - default: linear
 - `mipmap: Bool` Enables or disables mipmapping for the target.
    - default: false
 - `float: Bool` Changes the way data is stored in the target.
    - default: false


### Unique Uniforms and Varyings

```glsl
// The total number of vertices used. This uniform only applies to vertex shader stages.
uniform int vertexCount;

// The color of the element being drawn. Only available in the vertex shader.
out vec4 v_color;

// don't touch this
in vec2 position;
```

## Compute Shaders

```glsl
#version 430

layout(local_size_x = 2, local_size_y = 2) in;
layout(rgba32f) uniform image2D img_output;

uniform vec4 resolution;

void main() {
  // get index in global work group i.e x,y position
  ivec2 pixel_coords = ivec2(gl_GlobalInvocationID.xy);

  vec4 pixel = imageLoad(img_output, pixel_coords);
  pixel.rg = pixel_coords / resolution.xy;

  // output to a specific pixel in the image
  imageStore(img_output, pixel_coords, pixel);
}
```

Make sure that if you want to run a shader over an entire texture, that:
`local_size_(xyz) * dispatch.(xyz) == resolution.(xyz)`

### Required fields

A compute shader stage must contain the following fields

 - `cs: Path` Specifies the file name of the compute shader file.
 - `dispatch: [Int; 1-3]` Sets the number of dispatches.
 - `resolution: [Int; 1-3]` Sets the size of the target texture.

### Optional field

 - `target: String` Specifies the name of the render target.
   - default: probably will crash but might write to the screen framebuffer
   - note, this creates an `imageND` which is different from a `samplerND`.

## Images

```yaml
images:
   - path: "relative/to/cwd.png"
     name: "name_of_uniform_sampler_2D"
   - path: "second/image/path"
     name: "uniform_of_second_image"
```

Currently supports only static images. `png` and `jpeg` have been tested.

## Common Uniforms

```glsl
// current render target resolution
uniform vec4 resolution; // vec4(x, y, x/y, y/x)

// stage index
// may be useful for running the same shader multiple times
uniform int passIndex;

// time in seconds since program startup
uniform float time;

// deltaTime between now and the previous frame
uniform float delta;

// increases with time * BPM / 60
// BPM is controlled by tap tempo in control panel
uniform float beat;

// array of sliders, corresponding to the sliders in control panel
uniform float sliders[32];

// array of buttons, corresponding to buttons in control panel
// buttons[i] = vec4(intensity, since_last_on, since_last_off, count);
// intensity: NoteOn velocity and PolyphonicKeyPressure value
// since_last_on: time in seconds since last NoteOn event
// since_last_off: time in seconds since last NoteOff event
// count: integer count of how many times button has been pressed
uniform vec4 buttons[32];

// The raw samples taken from the default audio in.
// r contains the left channel (or the only channel if the input is mono)
// g contains the right channel
uniform sampler1D samples;

// Raw FFT output
// r/g channels same as samples
uniform sampler1D raw_spectrum;

// "nice" FFT, does some bucketing and EQ
// r/g channels same as samples
uniform sampler1D spectrum;

// instantaneous volume
// r contains average of L/R volume, or the volume of the single channel for mono
// g contains the L channel volume
// b contains the R channel volume
uniform vec3 volume;

// A 32x32x32 random noise texture.
// Note this texture is recalculated per pipeline load, so the pattern changes every time you recompile or reload a pipeline.
uniform sampler3D noise;
```
