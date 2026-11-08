# rust-tests

## Dual License

As per standards for Rust and Bevy projects, this project is distributed under the Dual Apache/MIT License.

## Compiling on linux

Additional package requirements for linux

Bevy requires the libasound2-dev package, build may fail without it 

`failed to run custom build command for 'alsa-sys v0.3.1'`

```
sudo apt install libudev-dev libasound2-dev
```

To run the elf game: `cargo run --bin elf_game`

## About

As a way to learn more Rust, this is a project using Bevy's 3D game engine. This is also using a basic agentic harness and automated workflow to implement much of the plumbing or tedious parts.