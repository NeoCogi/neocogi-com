# cargo-nc-com

Build an executable and its nc-com runtime/plugins as separate shared libraries,
then assemble their artifacts and resources beside the executable.

Install from this checkout with `cargo install --path tools/cargo-nc-com`.
After publication, users can install with `cargo install cargo-nc-com --locked`.

```sh
cargo nc-com build                      # Native debug assembly
cargo nc-com build --release
cargo nc-com build --toolchain nightly --release
cargo nc-com build --profile min-size   # Selects nightly automatically
cargo nc-com run --release -- app-argument
```

The specialized `min-size` preset currently targets native x86_64 Linux GNU. It
bundles its target JSON and supplies its Cargo profile and compiler settings; no
consumer alias or profile definition is required. Install `rust-src` for the
selected nightly toolchain first. Ordinary builds support native Linux, macOS,
and Windows; foreign targets are rejected because inspection loads native code.

The tool reads `[package.metadata.nc-com]`, `[workspace.metadata.nc-com]`, or a
standalone file passed with `--config`. `cargo nc-com plan` prints the selected,
validated configuration without compiling or loading component code.

```toml
[package.metadata.nc-com]
runtime = "nc-com-runtime"
plugins = [{ package = "my-plugin", features = ["dynamic-plugin"] }]
assets = [{ from = "assets", to = "assets" }]
```

String components select workspace packages or unambiguous packages in the
application's resolved dependency graph. For a resolved dependency, the tool uses
Cargo's fetched source directory as read-only input to a generated build project.
Feature choices remain explicit in our configuration. Plugins and the application
normally share Cargo's feature unification, so shared dependencies compile with
one compatible feature set. Multiple resolved versions require an explicit source.
Detailed components support
`package`, `path`, `version`, `registry`, `features`, `default-features`, and `isolated`.
Set `isolated = true` on a plugin that intentionally needs a different feature
graph. Its independent build can require additional dependency compilations.
Application settings live in `application` and support `package`, `bin`,
`features`, and `default-features`. The application package and binary are
inferred only when unambiguous. Paths are relative to the owning manifest or
standalone configuration file.

A standalone file contains the assembly directly, without the metadata prefix.
For example, after publishing the referenced component versions:

```toml
# nc-com.toml
application = { package = "my-app", bin = "my-app" }
runtime = { package = "nc-com-runtime", version = "=0.1.0" }
plugins = [{ package = "my-plugin", version = "=1.0.0", features = ["dynamic-plugin"] }]
assets = [{ from = "assets", to = "assets" }]
```

Build it with `cargo nc-com build --config nc-com.toml --release`. An explicit
`version` uses registry resolution even when a workspace member has the same name.
An explicit `path` selects local source; when both are supplied Cargo also checks
that the local package satisfies the version requirement. Components may expose
an ordinary Rust library (`rlib`/`lib`), both `rlib` and `cdylib`, or only `cdylib`.
For Rust libraries, the tool generates a small pure `cdylib` root that re-exports
the component. This preserves its public C exports and original crate identity
while allowing Cargo to apply the configured LTO. Existing cdylib-only components
are built directly. Rust `rlib` files and resolver executables are never staged.
Mixed-output implementations still emit their original `.so`/DLL as part of
Cargo's normal dependency build; the deployed output is the optimized root.

For multiple applications, use `default-assembly = "NAME"` and
`[assemblies.NAME]` inside the metadata/configuration table. Select another with
`--assembly NAME`. Package metadata takes precedence over workspace metadata;
`--config` takes precedence over both. `--package` selects package metadata and
overrides the chosen assembly's application package.

Named `variants.NAME` may replace `application-features`, append
`add-application-features`, replace `component-features.PACKAGE`, append
`additional-plugins`, remove `omit-plugins`, and replace `assets`. Repeated `--variant NAME` options
apply in command-line order. Unknown fields, component overrides, or variants
are errors. Asset destinations must stay within the generated assembly.

The CLI is a separate publishable Cargo package. Publishing requires its
versioned `nc-com-runtime` and `nc-com-core` dependencies to be available in the
registry first. This repository does not publish anything during a build.

The tool prepares the runtime, then builds the application and compatible plugins
in one Cargo invocation. It supplies `NC_COM_RUNTIME_LINK_DIR` for native linking
and uses Cargo's JSON messages to locate artifacts, including fresh cached
artifacts and custom library names. Explicitly isolated plugins build separately
and are snapshotted before the application can overwrite shared output filenames.
Phase messages say `Checking`; completion reports compiled and cached Cargo
targets, rather than implying that every phase recompiles its packages.
It preserves environment `CARGO_ENCODED_RUSTFLAGS` (or `RUSTFLAGS`) and adds native
loader paths. These environment flags take precedence over target rustflags in
Cargo configuration. On Linux the executable uses `$ORIGIN`; macOS uses
`@loader_path` with Rust's `@rpath` dylib identities. Windows uses adjacent DLLs
and the runtime's import library, including a conventional MSVC `.lib` alias.
Registry dependency artifacts may have hashed names: conventional runtime linker
aliases are staged alongside the originals, using hard links where available.
Separate debug-symbol files and directories are retained when Cargo reports them.

Component roots and the shared assembly resolver are generated Cargo projects.
Cargo resolves their versions and downloads their sources; our tool does not
implement a registry client or require nightly artifact dependencies. Generated
projects retain Cargo.lock under `sources/PACKAGE` and `sources/assembly` in the
managed state directory. Their initial version selections are seeded from the
application's Cargo.lock; compatible existing versions are retained when Cargo
adds the resolver packages. A changed application lock refreshes those seeds on
the next unlocked build. `--locked` requires current application and generated
locks and rejects a missing or changed seed, including the first build after
upgrading from an older resolver layout. Preserve the generated `sources`
directories for locked builds; cleaning the target directory removes them.
`--offline` applies to every Cargo invocation; `--frozen` combines both rules.
Workspace profiles, `[patch]`, and `[replace]` tables are preserved in generated
projects, with relative override paths resolved against the original workspace.

Outputs are managed under
`TARGET/nc-com/APPLICATION/ASSEMBLY/VARIANTS/dist/COMPILER/TARGET/PROFILE`.
The command prints the actual staging directory after success. Build state and
generated resolver manifests live alongside `dist`. The entire configured
runtime/plugin build shares Cargo's target directory and compatible compilation
cache. Cargo may compile different standard-library variants for separate graphs;
sharing a cache does not guarantee a single std compilation on a clean build.
The demo runtime and application declarations use the same default-feature setting
to avoid unnecessary runtime variants. Non-embedded demos do not build the asset
generator's host facade.
`--target-dir` overrides Cargo's configured target directory.

Each assembly contains the executable, runtime, plugins, configured assets, and
`plugins.txt`. A short-lived inspector process reads the trusted plugin descriptors
and uses nc-com's planner to reject missing dependencies, duplicate identities,
and cycles. Inspection can execute library initialization code. Exiting the
inspector releases its pinned libraries before directories are renamed on Windows.
Files are prepared in a temporary directory; build/inspection/copy failures leave
the previous assembly intact. Existing outputs require the tool's ownership marker
before replacement. Asset symlinks, traversal, and file collisions are rejected.
System dependencies such as SDL and graphics drivers are not bundled automatically.

`run` executes the staged binary with the caller's working directory, forwards
arguments after `--`, and preserves ordinary application exit codes. Applications
must locate their resources relative to their executable, as the demos do.

Validation commands from this repository checkout:

```sh
cargo test -p cargo-nc-com
cargo clippy -p cargo-nc-com --all-targets -- -D warnings
cargo test -p cargo-nc-com --test native nightly_min_size -- --ignored
```

The native tests build an external consumer, check actual dynamic runtime linking
and resource lookup after relocating the assembly and hiding the build cache,
exercise reordered descriptors and failed updates, and verify
versioned dependencies through an offline Cargo directory-source replacement.
The optional nightly test also checks reuse of core and std on a locked rebuild.
Native fixtures reference the checkout's ABI sources and are excluded from the
published package; the portable configuration tests remain included.
