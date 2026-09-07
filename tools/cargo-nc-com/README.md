# cargo-nc-com

Build an executable and its nc-com runtime/plugins as separate shared libraries,
then assemble their artifacts and resources beside the executable.

The tool reads `[package.metadata.nc-com]`, `[workspace.metadata.nc-com]`, or a
standalone file passed with `--config`. `cargo nc-com plan` prints the selected,
validated configuration without compiling or loading component code.

```toml
[package.metadata.nc-com]
runtime = "nc-com-runtime"
plugins = [{ package = "my-plugin", features = ["dynamic-plugin"] }]
assets = [{ from = "assets", to = "assets" }]
```

String components select workspace packages. Detailed components support
`package`, `path`, `version`, `registry`, `features`, and `default-features`.
Application settings live in `application` and support `package`, `bin`,
`features`, and `default-features`. The application package and binary are
inferred only when unambiguous. Paths are relative to the owning manifest or
standalone configuration file.

For multiple applications, use `default-assembly = "NAME"` and
`[assemblies.NAME]` inside the metadata/configuration table. Select another with
`--assembly NAME`. Package metadata takes precedence over workspace metadata;
`--config` takes precedence over both. `--package` selects package metadata and
overrides the chosen assembly's application package.

Named `variants.NAME` may replace `application-features`, append
`add-application-features`, replace `component-features.PACKAGE`, append
`additional-plugins`, and replace `assets`. Repeated `--variant NAME` options
apply in command-line order. Unknown fields, component overrides, or variants
are errors. Asset destinations must stay within the generated assembly.

The CLI is a separate publishable Cargo package. Publishing requires its
versioned `nc-com-runtime` and `nc-com-core` dependencies to be available in the
registry first. This repository does not publish anything during a build.
