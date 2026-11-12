# Serialization dynamic plugin

This crate only exports the native descriptor entry point. The implementation
lives in `neocogi-com-serialization`, which also supports static registration.
NCB is always available and selected by default. Enable `json` to include our
custom JSON format in the same shared library.

Use `cargo build --profile min-size -p neocogi-com-serialization-dylib --features json`
for the deployed artifact under `target/min-size`. The ordinary profile owns the
complete nightly/std-rebuild configuration; no wrapper command is required.
