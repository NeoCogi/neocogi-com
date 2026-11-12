# Serialization dynamic plugin

This crate only exports the native descriptor entry point. The implementation
lives in `neocogi-com-serialization`, which also supports static registration.
NCB and our custom JSON format are always built into this shared library,
including with `--no-default-features`. NCB is selected by default. There are
no format feature switches on the dylib crate.

Use `cargo build --profile min-size -p neocogi-com-serialization-dylib`
for the deployed artifact under `target/min-size`. The ordinary profile owns the
complete nightly/std-rebuild configuration; no wrapper command is required.
