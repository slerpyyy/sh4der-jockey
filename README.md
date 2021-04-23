# Sh4derJockey
*A custom VJ tool written by sp4ghet and slerpy*

![code-quality](https://img.shields.io/badge/code%20quality-jank-red)
![works-on](https://img.shields.io/badge/works%20on-my%20mashine%E2%84%A2-orange)

## Wishlist

A non-exhaustive list of things we want our tool to be able to do, vaguely ordered by priority, from high to low:

- [x] fullscreen fragment shaders
- [x] live shader reloading
- [x] dynamic / configurable render pipeline
- [ ] shader preprocessor / common file
- [ ] input system
    - [ ] MIDI control
    - [ ] tap-to-sync / BPM calculator
    - [ ] OSC? not really useful for me but maybe someone else
- [ ] audio reactive / FFT
- [x] mip maps
- [ ] code view / screen capture
    - [ ] NDI input? Direct Screen Capture?
- [ ] vertex shaders
    - [ ] loading models and textures
- [ ] geometry shaders
- [ ] compute shaders
    - [x] a compute shader runs
    - [ ] multiple render targets
    - [ ] work group size config
- [ ] hardware instancing
- [ ] curve editor
- [ ] color palette
- [x] performance profiler
- [ ] 3D noise texture
- [x] resizable window
- [ ] cubemaps
- [ ] text rendering
- [ ] custom textures/LUTs
    - [ ] videos
- [ ] recording mode

## Setup

Before building this project, make sure you have the **SDL2 development library** installed. You can find instructions on how to do that on the [rust-sdl2 repo](https://github.com/Rust-SDL2/rust-sdl2#requirements).

Once that's done, the rest should be muscle memory:
```sh
# clone the repo
git clone https://github.com/slerpyyy/sh4der-jockey.git
cd sh4der-jockey

# build and run
cargo run

# install so you can run it from anywhere
cargo install --path .
```
