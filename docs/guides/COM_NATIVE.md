# Native Cargo builds and browser composition

Cargo owns application compilation, dependency resolution, feature selection and
native artifact paths. There is one workspace lockfile. There are no generated
component workspaces, wrapper library packages, assembly variants or `plugins.txt`.
`cargo-neocogi` retains [module authoring commands](../../tools/cargo-neocogi/README.md).

## Build and run

The repository pins nightly-2026-09-29 in `rust-toolchain.toml` and enables Cargo
artifact dependencies in `.cargo/config.toml`. Rustup installs the selected tools.
Artifact dependencies currently require [nightly Cargo](https://doc.rust-lang.org/cargo/reference/unstable.html#artifact-dependencies). The COM ABI, macros and
IDL library retain their stable Rust 1.85 boundary; the isolated native linkage
crate uses nightly Rust. External stable consumers can use the SDK from their
own workspaces.

```sh
cargo run -p neocogi-demo-full
cargo run -p neocogi-demo-full --release --no-default-features --features dynamic,vulkan
cargo run -p neocogi-demo-basic --bin neocogi-demo-calculator
cargo run -p neocogi-demo-smoke -- --smoke
cargo run -p neocogi-theme-editor
cargo run -p neocogi-ui-designer
```

Native applications default to `dynamic,glow`. Cargo features are additive:
alternate renderers and monoliths require `--no-default-features`. Choose one
renderer and one linkage per build. The application manifest forwards selections
to its composition; the renderer owns the exclusive backend check.

```sh
cargo run -p neocogi-demo-full --no-default-features --features dynamic,wgpu
cargo run -p neocogi-demo-full --no-default-features --features monolith,glow,embed-assets
cargo run -p neocogi-demo-smoke --no-default-features --features dynamic-extension,glow -- --smoke
cargo run -p neocogi-demo-smoke --no-default-features --features monolith-extension,glow -- --smoke
```

`NEOCOGI_DEMO_FRAMES=8` selects a finite full/basic/editor run for acceptance.
The smoke command has its own deterministic `--smoke` behavior.

## Platform dependencies

Native renderers need the platform SDL2 development library and an available GPU
backend. Vulkan also links the platform Vulkan loader. CI installs SDL2, Vulkan
and software Mesa drivers on Linux; macOS uses Homebrew SDL2. Windows CI installs
`vcpkg install sdl2:x64-windows`, adds its library directory to `LIB` and its DLL
directory to `PATH`. Cargo owns NeoCogi artifacts; those OS dependencies retain
their usual platform installation requirements. Browser builds use WebGL2 and
require neither SDL nor native plugin artifacts.

## What Cargo builds

The implementation packages own real `rlib` and `cdylib` targets. Dynamic
composition requests only their `cdylib` artifacts. Static composition uses normal
Rust dependencies and registers their descriptor accessors.

The existing `neocogi-demo-host` handles application startup. Its optional
`neocogi-demo-plugins` dependency contains the native artifact declarations:

```toml
[dependencies]
ui-library = { package = "neocogi-ui-runtime", path = "../../crates/ui/runtime", artifact = "cdylib", features = ["dynamic-plugin"] }
```

That dependency has `lib = false`, Cargo's artifact default. It builds a DLL
without linking the provider's Rust library into the host. The generated path is
used directly in normal source:

```rust,ignore
let path = env!("CARGO_CDYLIB_FILE_UI_LIBRARY_nc_ui");
// SAFETY: this is the trusted artifact built from the selected Cargo graph.
unsafe { neocogi_com::Runtime::global().load_plugin_library(std::path::Path::new(path)); }
```

There is no generated startup file. Registration lists actual selected Cargo
artifacts in explicit provider order. Nodes requires UI runtime; Skin requires
GFX. Runtime registration validates those requirements without loading additional
libraries. Metadata inspection constructs no objects.

The composition crate exists because Cargo rejects naming the same provider twice
in one package as separate artifact and normal Rust dependencies. Keeping DLL
references in their own small crate permits the host's monolith/browser path to
use normal Rust providers. This crate neither invokes Cargo nor resolves packages.

## One shared runtime

Every dynamic consumer resolves process-wide registration, provenance, dependency
checks and pinned code through one `nc_com`. A DLL must never privately link a
runtime instance. `neocogi-com` is the typed facade; it owns no runtime state.

The native composition and each dynamic plugin use one small
`neocogi-native-runtime-link` integration crate. It declares `nc_com` as a normal
`artifact = "cdylib"` dependency, with no Rust implementation library attached.
Cargo therefore builds the deployed runtime using the target's selected profile,
including its optimization, panic strategy and standard library. Never declare
this deployed library under `build-dependencies`: Cargo's build-tool profile
otherwise silently selects unoptimized code and unwinding even in a size build.

```toml
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
neocogi-native-runtime-link = { workspace = true, optional = true }
```

A dynamic provider enables that dependency with `dynamic-plugin` and imports its
metadata using `use neocogi_native_runtime_link as _;`. This never embeds runtime
state. Cargo deduplicates the normal runtime artifact across consumers.

Normal artifact paths are available while compiling Rust, not to `build.rs`.
The integration crate converts Cargo's `CARGO_CDYLIB_DIR_COM_LIBRARY` value into
transitive native linker search metadata. Unix consumers also record that exact
build-tree directory as an rpath, so the executable and independently loaded DLLs
resolve the same runtime during direct execution without `LD_LIBRARY_PATH`.
Eight duplicated runtime-link build scripts are removed. No Cargo subprocess,
output-directory scan, copy, generated package or dependency resolver is involved.

This native-workspace bridge uses nightly Rust's `link_arg_attribute` in addition
to Cargo's nightly artifact dependencies. It is isolated from the portable
COM/IDL SDK and its stable Rust 1.85 consumers. On MSVC it supplies `/LIBPATH` for
Rust's `nc_com.dll.lib` import library, which the facade names
[verbatim](https://doc.rust-lang.org/reference/items/external-blocks.html#r-items.extern.attributes.link.modifiers.verbatim).
Windows still needs a DLL layout or loader environment at execution; Unix rpaths
do not provide a Windows deployment policy.

The runtime still verifies a requested root's exact IID, transitive signature and
threading before entering plugin code, then validates the returned owned
projection. Libraries remain pinned until process exit. Acquisition transfers one
reference; it promises neither a singleton nor stable identity between calls.

## Interfaces and implementation

NCIDL declares ordinary interfaces and values. A provider's creation methods are
ordinary interface methods with typed arguments. Concrete storage and plugin
publication live in its Rust implementation crate:

```rust,ignore
#[neocogi_com::com_object(api::IProvider)]
struct Provider;

#[neocogi_com::plugin(name = "example.provider")]
fn acquire_root() -> Result<neocogi_com::ComPtr<api::IProviderAbi>, neocogi_com::NcStatus> {
    Provider::from_value(Provider).query()
}
```

The return interface supplies root identity and threading. Version defaults to
Cargo package version; `dependencies = [PLUGIN_ID]` declares hard providers.
The macro generates `plugin_descriptor()` for static registration and the native
`nc_get_plugin_descriptor_v1` export under `dynamic-plugin`. No `class`/`factory`
IDL roles or factory bootstrap traits are required. A finished service is also a
valid plugin root. See [IDL modules](../contracts/IDL_MODULES.md) and the
[macro reference](../../crates/com/macros/README.md).

Consumers need generated interface definitions, not a Rust dependency on their
implementors. NCIDL imports read declarations at generation time. A public
construction method still belongs in an accessible contract; private storage and
bootstrap policy do not. Bindings can live in an API crate or an application's
`OUT_DIR`. Reuse the same Rust binding definitions when ordinary Rust helper APIs
exchange those types; independently generated definitions are nominally distinct.
Across DLLs, negotiation uses the matching C ABI contract instead.

## Resources and installed applications

Cargo demo compositions explicitly enable `neocogi-demo-assets/workspace-assets`.
Resources resolve from the source checkout recorded at compilation, independently
of the current working directory. Full-demo photo resources retain their actual
`demos/full/assets` location. `embed-assets` instead compiles the existing resource
bundle into the executable. Changing the current theme still uses the original
application asset base; it does not repeatedly derive from a prior theme.

Cargo artifact paths refer to this build tree. These development executables are
not relocatable installations. No automatic fallback chooses between an installed
DLL and a build-machine DLL. An installed application must explicitly choose its
library/resource layout, load trusted paths, and provide loader search policy.
A monolith with embedded resources avoids NeoCogi DLL/resource deployment; native
OS dependencies such as SDL remain. Distribution installation is separate from
compilation and is not supplied by the removed assembly command.

## Profiles and output

Use Cargo's ordinary `--release`, `--profile`, `--target` and `--target-dir` options.
Both release and min-size retain fat LTO, one codegen unit and symbol stripping.
The min-size profile also selects immediate abort, removes Rust panic locations
and debug formatting, and disables forced unwind tables for deployed target code.
Build scripts and procedural macros retain normal host panic/diagnostic settings.
Cargo's abort-test harness preserves the target panic strategy while isolating
panic expectations in child processes, including min-size profile tests.

```sh
cargo run -p neocogi-demo-full --no-default-features --features dynamic,vulkan --profile min-size
# The same build-tree executable can start directly on Unix:
./target/min-size/neocogi-demo-full
```

The workspace enables Cargo's `build-std` for std and its test/procedural-macro
support, preserves normal backtrace/unwind features, adds `optimize_for_size`, and installs
`rust-src` through the pinned toolchain. This is necessary because Cargo has no
profile-specific `build-std` switch: a named profile alone cannot rebuild std.
Target std is consequently rebuilt for other workspace profiles too, with an
initial compile/cache cost for each target/profile. Portable SDK consumers outside
this nightly development workspace keep their normal prebuilt standard library.
The `cargo build-min-size` alias is only shorthand for `cargo build --profile min-size`;
there is no separate target, wrapper, output layout or hidden compiler policy.

A dynamic artifact request builds only its requested cdylib target. A normal Rust
dependency on a package advertising both rlib/cdylib may also emit an unused shared
library during monolith/browser builds; removing that extra link would require
separate static/dynamic target packages or another crate-type selection mechanism.

Short native target names remain `nc_com`, `nc_ui`, `nc_ui_nodes`, `nc_render`,
`nc_gfx`, `nc_skin` and optional `nc_ui_extension`. The executable is in the usual
`target/<profile>` directory when no explicit target is selected. Dependency DLLs
remain in Cargo's artifact directories; they are not copied next to the executable.
Registration embeds Cargo's exact provider paths and native linkage records the
shared runtime directory. Do not reconstruct hashes or cache paths across builds.
An explicit `--target` adds Cargo's normal target-specific output directory.

## Browser

Browser builds use normal Rust dependencies, one static runtime, linked plugin
descriptors and embedded resources. They select neither native composition nor
native runtime artifact dependencies:

```sh
cargo check -p neocogi-demo-full --target wasm32-unknown-unknown --no-default-features --features webgl
scripts/web-demo.sh build
```

The existing web script performs browser binding generation and site preparation;
it does not build a second native Cargo graph.

## Validation

`scripts/test-demo.sh`, `scripts/test-extension.sh` and
`scripts/test-theme-editor.sh` execute normal Cargo builds and application tests.
They do not stage native assemblies. Cross-compiler and relocation tests for the
standalone COM sample remain separate from these development-composition checks.
The [migration record](../validation/CARGO_PLUGIN_SIMPLIFICATION_2026-10-15.md)
records fresh evidence and platform limitations.
