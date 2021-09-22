<img align="left" style="height: 17ch; margin: 1ch" src="docs/logo.png">

# Sh4derJockey
*A tool for shader coding and live performances*

<br>

## Documentation

The documentation on how to use this tool can be found in the [docs](docs/) folder or using the links below:

[Read in English](docs/readme_en.md) | [日本語で読む](docs/readme_jp.md)

## Setup

To build this project from source, you will need a Rust compiler and the Cargo package manager.
We highly recommend installing `rustup` which takes care of installing and updating the entire Rust toolchain.

Checkout the [Getting Started](https://www.rust-lang.org/learn/get-started) section on the rust-lang website for more.

```sh
# clone the repo
git clone https://github.com/slerpyyy/sh4der-jockey.git
cd sh4der-jockey

# build and run
cargo run

# install so you can run it from anywhere
cargo install --path .
```

| ⚠️ | Please note that the tool drops config files in the folder where the executable is located. |
|-|-|

It is up to the user to ensure that additional files in the install directory do not interfere with other programs.
The tool does also work when placed into a read-only directory, but user comfort will suffer.

## License

This project is licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

This program makes use of [NDI®](https://www.ndi.tv/) (Network Device Interface), a standard developed by [NewTek, Inc](https://www.newtek.com/).

Please refer to https://www.ndi.tv/ for further information about this technology.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
