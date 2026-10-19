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
and staging crates use nightly Rust. External stable consumers can use the SDK from their
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
without linking the provider's Rust library into the host. A compiler-only macro
copies the selected artifact and emits only its platform filename:

```rust,ignore
let filename = neocogi_native_layout::stage_library!("CARGO_CDYLIB_FILE_UI_LIBRARY_nc_ui");
let executable = std::env::current_exe()?;
let path = executable.parent().unwrap().join("lib").join(filename);
// SAFETY: this is the trusted artifact built from the selected Cargo graph.
unsafe { neocogi_com::Runtime::global().load_plugin_library(&path); }
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
The private `neocogi-native-layout` macro reads those paths during compilation
and copies the runtime into the application's `lib/`. The linkage crate's small
build script supplies that directory for linking and Cargo's test loader environment;
it never declares the runtime as a build dependency. Linux runtime lookup uses
`$ORIGIN/lib:$ORIGIN`; macOS uses `@loader_path/lib` and `@loader_path`, with
`@rpath` library identities. These relative paths serve both executables and
independently loaded plugins. No build-machine directory enters runtime lookup.

This native-workspace bridge uses nightly Rust's `link_arg_attribute` and tracked
procedural-macro inputs in addition to Cargo's nightly artifact dependencies.
It is isolated from the portable COM/IDL SDK and its stable Rust 1.85 consumers.
On Windows it copies `nc_com.dll` beside the executable for initial imports;
compile-only import libraries stay in `lib-imports/`. The facade names MSVC's
`nc_com.dll.lib` import library
[verbatim](https://doc.rust-lang.org/reference/items/external-blocks.html#r-items.extern.attributes.link.modifiers.verbatim).
Platform execution evidence is recorded separately; Linux checks do not establish
Windows or macOS loader behavior.

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

Native file-backed resources are copied during compilation into `assets/` and
`themes/` beside the executable. The full-demo photo is copied to
`assets/FACEPALM.png`. Runtime resource paths start from `current_exe()`;
the working directory and source checkout are irrelevant. Relative smoke-test
`--theme` paths use that same base; explicit absolute user paths remain supported.
`embed-assets` compiles the existing resource bundle instead and omits resource
staging. Theme changes retain the original application asset base.

The resulting layout is:

```text
target/<profile>/
  neocogi-demo-full[.exe]
  nc_com.dll                 # Windows initial import only
  lib/                       # shared runtime and selected plugin libraries
  assets/                    # file-backed resources, unless embedded
  themes/                    # file-backed themes, unless embedded
```

Copy these entries together to relocate an application. Do not copy the whole
Cargo target directory: `lib-imports/`, compiler caches and debug unremapping
sidecars are compilation inputs, not distribution files. Workspace profiles use
Cargo's `trim-paths = "all"` so panic/debug/compiler source locations do not
disclose checkout or Cargo-home paths. **`*.trim-paths.json` sidecars deliberately
contain original paths for local debugging and must stay on the build machine.**
Compiler path trimming does not sanitize arbitrary application string literals;
artifact paths therefore never appear in generated runtime code.

Staging reads exactly Cargo-selected artifact paths and literal resource inputs;
it invokes no Cargo subprocess and constructs no dependency graph. It tracks
source files, resource directory membership and copied files in compiler dep-info.
Deleted outputs and cached renderer switches trigger copying again. Identical
contents retain their timestamps; changed libraries are replaced atomically,
without truncating a mapped Unix library. An in-use Windows DLL can reject
replacement: stop the application before rebuilding. Removed optional providers
or resources may leave unused files in a shared profile directory; registration
loads only the compiled selection. Use a clean profile for distribution.
Restoring outputs during macro expansion can require one following compilation
to settle Cargo's timestamp-based dep-info; subsequent unchanged builds are fresh.

The pinned Cargo version marks each profile output directory with `.cargo-lock`.
Staging locates that marker among ancestors of Cargo's artifact or `OUT_DIR` path;
it never reconstructs hashes or infers a second graph. With a separate Cargo
build directory, set the absolute build-only `NEOCOGI_NATIVE_OUTPUT_DIR` to the
actual executable directory. Unsupported layouts fail compilation explicitly.
This private integration is tied to the pinned Cargo layout. Distribution
installation, signing and platform SDL/Vulkan deployment remain separate tasks.

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
# The executable uses the adjacent lib/ and resource directories:
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
remain in Cargo's artifact directories as compiler inputs, with deployment copies
in `lib/`. Registration embeds filenames and native linkage uses relative paths.
Do not reconstruct hashes or cache paths across builds.
An explicit `--target` adds Cargo's normal target-specific output directory.

## Debugging in VS Code

Open this repository root as the workspace folder and install its recommended
`rust-lang.rust-analyzer` and `vadimcn.vscode-lldb` extensions. In Run and Debug,
select **Full demo: Vulkan (shared, debug)** or **Full demo: Glow (shared, debug)**
and press F5. CodeLLDB invokes the ordinary Cargo build and selects its reported
binary; no target/cache path is reconstructed. These configurations use `dev`,
which retains unoptimized code and debug information. Release/min-size remain
stripped and are unsuitable for source stepping.

The workspace settings also select CodeLLDB for rust-analyzer's Debug action.
Before launching an application, CodeLLDB imports
[`scripts/lldb_source_map.py`](../../scripts/lldb_source_map.py). It reads the
selected executable's adjacent `.trim-paths.json`, the actual version-1 mapping
format emitted by the pinned Cargo. The file supplies the inverse mappings for
workspace files, `OUT_DIR` bindings, registry dependencies and rebuilt std; the
debugger appends those mappings after any explicit source-map rules. This avoids
hard-coded Cargo prefixes, registry hashes, toolchain names and target directories.
An untrimmed executable without a sidecar needs no mappings. Unsupported or
malformed metadata produces a debugger import error.

Set breakpoints directly in provider Rust source before starting. They can remain
unresolved until the provider DLL loads; CodeLLDB then binds them and supports
ordinary stepping and local-variable inspection. For example, a breakpoint on
the call to `create_runtime_from_assets` in `crates/ui/runtime/src/lib.rs` stops
inside the copied `libnc_ui.so`. Generated COM client/thunk frames should open
their Rust file under `OUT_DIR`, rather than disassembly.

The mapping file stays on the build machine and is not copied into `lib/`.
Debug symbols in the copied ELF libraries are preserved. The native acceptance
test scripts still use their copied test layout; launching an acceptance test's
compiler-directory binary directly does not supply adjacent resources. Separate
PDB/dSYM staging and Windows/macOS debugger execution are not covered by the
Linux checks alone. See the [current debugger validation scope](../validation/README.md#tooling-and-debugger-metadata).

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
The build itself copies its selected files. `scripts/test-native-layout.py --cache
--run` checks actual Linux binaries, cached backend switches, deleted outputs and
a renamed full application directory. The plugin regression also relocates its
test host and calls a separately built DLL without loader environment overrides.
Cargo nests unit-test executables in compiler directories. The acceptance scripts
use `scripts/run-native-test.py` as a Cargo runner to copy an already compiled
test and its staged inputs into a temporary application directory. This test-only
runner never builds; production startup always uses adjacent files directly.
Cross-compiler tests for the standalone COM sample remain separate.
The [current native validation guide](../validation/README.md#renderers-and-native-applications)
lists maintained entrypoints and platform limitations.
