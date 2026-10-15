# Native Cargo builds and browser composition

Cargo owns application compilation, dependency resolution, feature selection and
native artifact paths. There is one workspace lockfile. There are no generated
component workspaces, wrapper library packages, assembly variants or `plugins.txt`.
`cargo-neocogi` retains [module authoring commands](../../tools/cargo-neocogi/README.md).

## Build and run

The repository pins nightly-2026-09-29 in `rust-toolchain.toml` and enables Cargo
artifact dependencies in `.cargo/config.toml`. Rustup installs the selected tools.
Artifact dependencies currently require [nightly Cargo](https://doc.rust-lang.org/cargo/reference/unstable.html#artifact-dependencies). The COM ABI, macros and
IDL library contain no nightly Rust language features; external stable consumers
can use them from their own workspaces.

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

The native composition and each dynamic plugin declare the same runtime artifact
as a build dependency. Cargo deduplicates its build and supplies its directory:

```toml
[build-dependencies]
com-library = { package = "neocogi-com-runtime", path = "../../crates/com/runtime", artifact = "cdylib", target = "target" }
```

A plugin gates that dependency with `dynamic-plugin`; static/browser builds leave
it disabled. The build script only consumes `CARGO_CDYLIB_DIR_COM_LIBRARY` and
prints a native link search directory. It does not build, locate or copy runtime
binaries. This linkage belongs at native boundaries because Cargo may unify the
host's facade `runtime` feature into provider dependencies. Even a provider that
currently only creates objects must then be able to resolve that shared import.
Reusable ABI/compiler crates have no artifact dependencies.

On MSVC the facade names Rust's `nc_com.dll.lib` import library
[verbatim](https://doc.rust-lang.org/reference/items/external-blocks.html#r-items.extern.attributes.link.modifiers.verbatim). Linux
and macOS use the corresponding ordinary `nc_com` native library name. Cargo
supplies loader search paths when executing `cargo run` and `cargo test`.

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
Release fat LTO now runs at each implementation's real cdylib target. A dynamic
artifact request builds only that requested target type. A normal Rust dependency
on a package advertising both `rlib` and `cdylib` may also emit an unused shared
library during monolith/browser builds; removing that extra link would require
separate static/dynamic target packages or another crate-type selection mechanism.
The migration removes the custom builder rather than hiding that Cargo tradeoff.

Short native target names remain `nc_com`, `nc_ui`, `nc_ui_nodes`, `nc_render`,
`nc_gfx`, `nc_skin` and optional `nc_ui_extension`. Their actual artifact directories
are Cargo-owned. Do not reconstruct hashed paths in scripts or cache them across
builds. The workspace's `cargo build-min-size` alias remains an explicit nightly
size experiment; it is separate from the normal development workflow.

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
