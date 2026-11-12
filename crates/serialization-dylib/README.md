# Serialization dynamic plugin

This crate only exports the native descriptor entry point. The implementation
lives in `neocogi-com-serialization`, which also supports static registration.
NCB is always available and selected by default. Enable `json` to include our
custom JSON format in the same shared library.

Use `cargo build-min-size --features json` for the deployed artifact. It selects
the pinned nightly/std-rebuild configuration and reports the actual library path.
The plain stable Cargo profile alone does not rebuild its precompiled std.
