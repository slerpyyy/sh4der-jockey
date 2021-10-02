# Sh4der Jockey
*A tool for shader coding and live performances*

## Getting started

First put the executable in a place where it is allowed to create new files. It currently generates only a single file to store the window layout of the control panel, but this might change in the future.

Then run the tool in your project folder with the `init` flag. This will instruct the tool to set up a simple example project.

## UI

You can bind buttons and sliders to MIDI buttons and sliders by holding the `bind` button while moving the slider or hitting the button. The last note before the button is released will be bound to that button/slider.

## Config File
A config file is a special yaml file called `config.yaml` at the project root alongside the pipeline files (described below). This configures certain things for the project as a whole, which spans several pipelines.
Without a config file, the program defaults to collecting all MIDI inputs and the default audio input.
An example config file is shown below:

```yaml
midi_devices:
  - "My MIDI Substr"
  - "Other Device"
audio_device: "Audio Input Substr"
```

This will search for the relevant MIDI and audio devices based on a simple matching based on `device_name.contains(substr)`.

## Pipeline

Once the tools is starts, it looks for files ending in `.yaml` in the current working directory and treats these as pipeline files.

Below is an example pipeline file.
You can have multiple pipelines in the working directory and choose from the Control Panel.
```yaml
stages:
  - cs: "particle_pos.comp"
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

  - fs: "post_process.frag"
    uniforms:
      - chromab: 0.4

ndi:
  - source: "source substring"
    name: "sampler_name"

images:
  - path: "images/image.png"
    name: "some_image"

audio:
  audio_samples: 8192
  spectrum:
    mipmap: true
    filter: linear
    wrap_mode: repeat
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
uniform int vertex_count;

void main(){
  float a = 12 * gl_VertexID / vertex_count;
  gl_VertexPos = vec4(cos(a), sin(a), a, 1);

  v_color = vec4(1);
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
uniform int vertex_count;

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
 - `target: String` Specifies the name of the render target.
   - note, this creates an `imageND` which is different from a `samplerND`.

## Images

```yaml
images:
   - path: "relative/to/cwd.png"
     name: "name_of_uniform_sampler_2D"
   - path: "second/image/path"
     name: "uniform_of_second_image"
```

```glsl
uniform sampler2D {name_of_image};
uniform vec4 {name_of_image}_res; // vec4(x, y, z, x/y)
```

Currently supports only static images. `png` and `jpeg` have been tested.

## Audio Config

```yaml
audio:
  audio_samples: int
  spectrum:
    mipmap: bool
    filter: (linear | nearest)
    wrap_mode: (clamp | repeat)
  raw_spectrum:
    mipmap: bool
    filter: (linear | nearest)
    wrap_mode: (clamp | repeat)
  samples:
    mipmap: bool
    filter: (linear | nearest)
    wrap_mode: (clamp | repeat)
```

All audio textures are float textures.

## Common Uniforms

```glsl
// current render target resolution
uniform vec4 resolution; // vec4(x, y, x/y, y/x)

// stage index
// may be useful for running the same shader multiple times
uniform int pass_index;

// time in seconds since program startup
uniform float time;

// Δt between now and the previous frame
uniform float time_delta;

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

// A 32x32x32 random noise texture.
// Note this texture is recalculated per pipeline load,
// so the pattern changes every time you recompile or reload a pipeline.
uniform sampler3D noise;

// current frame since program start
uniform int frame_count;

// The raw samples taken from the default audio in.
// r contains the left channel (or the only channel if the input is mono)
// g contains the right channel
uniform sampler1D samples;

// Raw FFT output
// r/g channels same as samples
uniform sampler1D spectrum_raw;

// "nice" FFT, does some bucketing and EQ
// r/g channels same as samples
uniform sampler1D spectrum;
uniform sampler1D spectrum_smooth;
uniform sampler1D spectrum_integrated;
uniform sampler1D spectrum_smooth_integrated;

// Bass/Mid/High
uniform vec3 bass;
uniform vec3 bass_smooth;
uniform vec3 bass_integrated;
uniform vec3 bass_smooth_integrated;

uniform vec3 mid;
uniform vec3 mid_smooth;
uniform vec3 mid_integrated;
uniform vec3 mid_smooth_integrated;

uniform vec3 high;
uniform vec3 high_smooth;
uniform vec3 high_integrated;
uniform vec3 high_smooth_integrated;

// instantaneous volume
// r contains average of L/R volume, or the volume of the single channel for mono
// g contains the L channel volume
// b contains the R channel volume
uniform vec3 volume;
uniform vec3 volume_integrated;
```

Additionally, custom uniforms can be added to any shader stage using the `uniforms` field in the pipeline file.

```yaml
stages:
  - fs: "scene.frag"
    uniforms:
      - strength: 2.3
      - iter: 20
      - color: [1, 0.4, 0.7]
      - rotation^T: [[0.9, 0.2], [-0.2, 0.9]]
```

These can be accessed in the shader as follows.
Note that all numbers in custom uniforms are represented as floats, to make the type of a uniform easy to infer.

```glsl
uniform float strength;
uniform float iter;
uniform vec3 color;
uniform mat2 rotation;
```

Also note that the `rotation` matrix is transposed here.
By default, a matrix is interpreted in row major order. If they are transposed, they are interpreted in column major order.

## Hotkeys

|key combination| feature |
| --- | --- |
| ctrl + enter | rebuild current pipeline |
| alt + enter | Toggle borderless fullscreen |
| shift + ctrl + s | take screenshot and save it in the cwd |
