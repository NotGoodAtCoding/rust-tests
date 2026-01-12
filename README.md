# rust-tests

## Compiling on linux

Additional package requirements for linux

Bevy requires the libasound2-dev package, build may fail without it 

`failed to run custom build command for 'alsa-sys v0.3.1'`

```
sudo apt install libudev-dev libasound2-dev
```