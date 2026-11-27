# Serialization dynamic plugin

This crate only exports the native descriptor entry point. The implementation
lives in `neocogi-com-serialization`, which also supports static registration.
NCB and our custom JSON format are always built into this shared library,
including with `--no-default-features`. NCB is selected by default. There are
no format feature switches on the dylib crate.

From the SDK checkout, build and stage the native plugin into your deployment
directory with the configured min-size profile:

```sh
cargo xtask stage-serialization ./deploy
```

| Platform | Cargo library | Deployed filename |
| --- | --- | --- |
| Linux | `libnc_serialization.so` | `libnc-serialization.so` |
| macOS | `libnc_serialization.dylib` | `libnc-serialization.dylib` |
| Windows | `nc_serialization.dll` | `nc-serialization.dll` |

If you only have the crate archive, build it with `cargo build --release --locked`
and copy the Cargo library to the deployed filename for your platform. Pass the
deployed path to the host's plugin loader.
