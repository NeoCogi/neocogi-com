# Native COM packages and browser assembly

Install the command from this checkout and build a runnable assembly:

```sh
cargo install --path tools/cargo-neocogi
cargo neocogi build --assembly full
cargo neocogi run --assembly full
```

Contracts generate Rust bindings and source-graph metadata through ordinary
Cargo build scripts. They produce no native libraries or description archives.
`cargo-neocogi` is optional: the [sample plugin](../../examples/com-plugin/README.md)
has complete plain-Cargo commands for dynamic and monolithic deployment.

`cargo neocogi` builds the runtime, selected plugins, and application in order,
then stages the executable, native libraries, plugin manifest, and assets. It
reads the assembly definitions in `[workspace.metadata.neocogi]` in the root
[Cargo.toml](../../Cargo.toml). Users of a published tool can instead install with
`cargo install cargo-neocogi --locked`; publishing the tool and its component
prerequisites is a separate release operation.

`--assembly` selects `full` (the default), `simple`, `calculator`,
`backend-frame-cube`, `smoke`, or `theme-editor`. Glow is the default backend. Renderer variants
select matching application and platform features explicitly:

```sh
cargo neocogi run --assembly full --variant vulkan --release
cargo neocogi run --assembly full --variant vulkan --variant monolith --release
cargo neocogi run --assembly calculator --variant wgpu
cargo neocogi run --assembly smoke --variant extension -- --smoke
```

These commands build selected packages rather than the entire workspace, so a
Glow build does not select the Vulkan/WGPU renderer packages. `--workspace` on
ordinary Cargo still selects every renderer root, including their mandatory
Ash/WGPU dependencies; `--no-default-features` does not deselect those packages.

For contributors, the equivalent command without installation is
`cargo run -p cargo-neocogi -- build --assembly full`. The installed command needs
no workspace alias. Run `cargo neocogi plan --assembly full --variant vulkan` to
review the selected typed configuration without compiling or inspecting plugins.
The [tool reference](../../tools/cargo-neocogi/README.md) describes package metadata,
standalone `--config` files, per-component features, registry version requirements,
resource paths, and lockfile behavior.

Named dependency components are selected only from packages reachable by Cargo
edges from the chosen application's exact package ID, including renamed and
transitive dependencies under its configured features. A sibling application's
different version cannot create ambiguity or supply an otherwise missing
component. Multiple versions actually reachable from the selected app require
an explicit source. Workspace components and explicit path/version declarations
remain intentional selection routes. One original metadata resolution supplies
all component lookups in an operation.

The theme editor stages UI core, built-in nodes, the selected renderer, GFX with
its authoring builder, one shared COM runtime, and adjacent assets/themes. Its
`monolith` variant links and registers those same providers directly; `embedded`
selects compiled default assets. It shares document/preview code with WebAssembly:

```sh
cargo neocogi run --assembly theme-editor --variant vulkan
cargo neocogi run --assembly theme-editor --variant vulkan --variant monolith
NEOCOGI_WEB_APP=theme-editor ./scripts/web-demo.sh serve
```

[`scripts/test-theme-editor.sh`](../../scripts/test-theme-editor.sh) exercises either
native linkage mode from relocated staging with exact staged resources. See the
[editor guide](../../tools/theme-editor/README.md) for project I/O and browser imports.

## Profiles and output

Cargo package names remain descriptive; each deployable component declares a
short `[lib] name`. The assembly tool preserves that target name, including for
third-party plugins. Components staged together must use distinct library target
names. Direct `cargo rustc --lib --crate-type cdylib` builds use
the same filenames. Rust code that directly imports an implementation uses its
library target name; application facades retain their `neocogi_*` names.

| Cargo package | Rust library target | Linux shared library |
| --- | --- | --- |
| `neocogi-com-runtime` | `nc_com` | `libnc_com.so` |
| `neocogi-ui-runtime` | `nc_ui` | `libnc_ui.so` |
| `neocogi-ui-nodes` | `nc_ui_nodes` | `libnc_ui_nodes.so` |
| `neocogi-render-plugin` | `nc_render` | `libnc_render.so` |
| `neocogi-gfx` | `nc_gfx` | `libnc_gfx.so` |
| `neocogi-skin` | `nc_skin` | `libnc_skin.so` |
| `neocogi-example-ui-extension` | `nc_ui_extension` | `libnc_ui_extension.so` |
| `neocogi-example-com-plugin` | `nc_example_com` | `libnc_example_com.so` |

On macOS these become `libnc_*.dylib`; on Windows they become `nc_*.dll`
with their generated import libraries. MSVC runtime linkage also receives the
`nc_com.lib` alias when rustc produces `nc_com.dll.lib`.
Each assembly selects one renderer, so Glow, Vulkan, and WGPU variants all
deploy `libnc_render.so` in separate variant directories. Persistence lives in
`libnc_com.so`. Contracts and support crates do not produce deployed libraries.

Rebuild and restage existing assemblies after the naming change. Native hosts
now link `nc_com`, and generated `plugins.txt` files name the new artifacts.
If using an installed command, update it from this checkout with
`cargo install --path tools/cargo-neocogi --force` before rebuilding.
Monoliths and browser modules still link the implementations statically.

```sh
cargo neocogi build --assembly full                       # Debug
cargo neocogi build --assembly full --release
cargo neocogi build --assembly full --toolchain nightly --release
rustup component add rust-src --toolchain nightly
cargo neocogi build --assembly full --profile min-size
```

The `min-size` preset selects nightly automatically and includes its x86_64 Linux
GNU target JSON, size-oriented compiler settings, and rebuilt standard library.
Consumers do not need our `build-min-size` alias or a custom profile definition.
The preset omits generated `Debug` formatting; shipped demo entry points therefore
report [`HostError`](../../demos/host/src/lib.rs) through its explicit `Display`
implementation so runtime failures still include their status or diagnostic text.
Frame errors identify the rejecting host operation (such as event/layout update,
IME placement or rendering) and retain the original COM status. Native demo theme
loading also prints Skin's owned diagnostic before releasing its short-lived
loader, including the theme and source path. Renderer detail is attached only
to `UI_E_RENDER`, avoiding unrelated stale backend messages.
Generated contract bindings compile in the same Cargo target graph as their
consumers; build scripts do not invoke a separate native compiler for contracts.
Use `--toolchain nightly-YYYY-MM-DD` to select an installed dated toolchain. Custom
nightly targets can change with the compiler, so pin and test the toolchain used
for a release. Ordinary custom profiles remain defined by the application's
workspace. Compatible plugins and the application build in one Cargo feature
graph after preparing the runtime. Generated pure cdylib roots re-export Rust
implementations, allowing the shared libraries to receive the profile's LTO.
Existing cdylib-only components build directly. `isolated = true` on a plugin
retains an independent feature graph where required. Identical settings share
Cargo's cache; genuinely different graphs can still require different std artifacts.

The command prints the completed output directory. Its layout is:

```text
TARGET/neocogi/APPLICATION/ASSEMBLY/VARIANTS/dist/COMPILER/TARGET/PROFILE/
```

`TARGET` at the start is Cargo's configured build directory, or `--target-dir`.
`COMPILER` identifies the selected compiler; the second `TARGET` identifies the
compilation platform. `dev` outputs use the `debug` directory name. Keep every
staged runtime/plugin library and resource with the executable. `run` launches
the staged binary and forwards arguments after `--`, preserving its exit code.

UI assemblies contain two adjacent provider libraries: `libnc_ui`
publishes the core retained runtime and `libnc_ui_nodes` publishes concrete
built-in nodes. The manifest orders core before nodes because `neocogi.ui.nodes.runtime` declares a
hard provider dependency on `neocogi.ui.runtime`. Each library exports exactly one plugin getter;
their roots and implementations live in `crates/ui/runtime` and `crates/ui/nodes` respectively.
The non-plugin `neocogi-ui-support` resource/presentation helpers are not staged as a third UI
library. Nodes never links core implementation code; its declared provider dependency is resolved
through COM registration and its generated public host/layout/painter capabilities.

Add `--variant monolith` after the renderer selection to statically link the COM
runtime and the UI, renderer, GFX, and skin implementations. These assemblies
register the linked descriptors directly with the same SHA-256 ABI validation
and stage only the executable and configured resources. They therefore have no
Neocogi shared-library or `plugins.txt` deployment requirement. The assembly's
remaining per-component features are enabled on those exact packages in the
shared Cargo graph; for example, an external-resource monolith retains GFX's
`builder` capability even though the host dependency only selects the GFX crate.

The command discovers artifacts from Cargo's JSON output and snapshots the runtime
and explicitly isolated plugins before later builds can replace their files.
Grouped plugins are collected from the application's Cargo invocation. The phase
summary distinguishes compiled targets from cache hits. Runtime linker aliases handle hashed registry
artifacts; originals are retained for platform loader identities. A child process
inspects trusted plugin descriptors and uses neocogi-com's planner to reject missing
dependencies, duplicate identities, and cycles. It exits before staging directories
are renamed, releasing Windows library handles. Failed compilation, inspection,
or resource copying preserves the previous completed assembly.

On Linux `$ORIGIN` locates the adjacent runtime without `LD_LIBRARY_PATH`. macOS
uses `@loader_path`; Windows uses adjacent DLLs and the appropriate import
library. The tool preserves environment rustflags and adds these loader settings.
SDL 2.26+ and the selected graphics API's system libraries remain system
prerequisites; they are not automatically redistributed. Foreign targets are
rejected because native descriptor inspection cannot execute their libraries.
Linux is locally verified; CI exercises macOS and Windows independently.

## Resources and component boundaries

The default assemblies include the GFX and skin plugins plus `assets` and `themes`.
Full-demo packaging also places its photograph at `assets/FACEPALM.png`. Inputs
resolve relative to the owning manifest/configuration file; staged paths resolve
relative to the application executable. Theme parsing runs in the skin plugin;
image decoding and font rasterization run behind the GFX boundary. Call
`neocogi_demo_host::initialize()` before `load_bundle()`.

```sh
cargo neocogi run --assembly full --variant vulkan --variant embedded
cargo neocogi run --assembly smoke -- --smoke --theme themes/windows-95/theme.json
```

Place `--variant embedded` after the renderer variant. It adds the application's
`embed-assets` feature and removes external resources. The full demo retains its
PNG-capable GFX plugin and live style compiler. Other embedded demos keep the compact
GFX compositor but omit its builder feature and the optional Skin plugin. The smoke assembly's
`extension` variant adds the independent example plugin. Browser `webgl` selects
embedded resources and retains its separate static registration workflow.

Native demos declare runtime/plugin crates as ordinary dependencies. Dynamic
startup uses only their C ABI and loads staged plugin files. Monolithic startup
retains those same implementations by directly registering their descriptors.
An ordinary dynamic demo `cargo build` still lacks a runtime linker search path
and staged assembly. The command supplies that path for dynamic builds or selects
the application-owned static graph for a monolith.
`neocogi-ui` remains the static typed facade without native UI/renderer
implementation dependencies. See the [backend source map](../../crates/ui/runtime/docs/BACKENDS.md#backend-sources).

Package metadata is our tool's configuration, not a Cargo execution hook. Ordinary
`cargo build` and `cargo install` do not automatically run the assembly command.
The generic tool supports versioned registry components using generated Cargo
projects, but this checkout's pre-alpha plugins are not thereby published.
Generated component and assembly lockfiles live under the target directory and
are initially seeded from the application's lockfile. Retain their `sources`
directories when using `--locked`, since `cargo clean` removes them. Changes to
the application lock require an unlocked build to refresh the generated locks.

## Assembly driver

`cargo neocogi` is the only native assembly path. It reads the workspace assembly
catalog, builds the executable and selected components, validates plugin descriptors,
and prints the final staging directory:

```sh
cargo neocogi build --assembly full --variant glow
cargo neocogi run --assembly full --variant glow
```

Use `--release` for the release profile or `--profile min-size` for the bundled
nightly size profile. The tool owns its layout under `target/neocogi`; callers and
scripts should use the reported `Staged ...` directory rather than reconstructing
that internal path.
COM_PERFORMANCE.md records whole-deployment measurements;
[FEATURES.md](FEATURES.md) records current deployment feature ownership.

## Native verification

A finite ordinary demo run renders the requested number of successful frames and
then releases its application session and host:

```sh
NEOCOGI_DEMO_FRAMES=8 cargo neocogi run --assembly full --variant glow
```

Malformed or zero limits fail at startup. Omission runs until OS close or the
application's `AppState::frame` returns false. Frame errors propagate to the native
entry point. The host closes the UI logically before releasing its capabilities;
retained widget handles then report `NC_E_DROPPED`.

`AppState::input` observes native polling, which may run ahead of UI routing.
Use `AppState::input_boundary` for ownership tied to a consumed input: Begin/End
bracket actual routing and its descendant/application messages outside traversal
borrows. This hook records bookkeeping only. Retain markers beside owned event
snapshots when decoding later, as the theme editor does for Undo groups. The host
continues to use one outer UI update. `IApplicationDispatcher` version 3 adds this
hook, so rebuild native application and UI providers together.

```sh
scripts/test-demo.sh glow
scripts/test-demo.sh vulkan
scripts/test-demo.sh wgpu
```

These tests build plugins independently, stage a companion test executable, and run
outside the repository. They exercise all four actual themes, image command submission, custom
geometry/mesh rendering, spacing changes, combo selection, Unicode log
submission and retained-handle closure. Win 3.11 is additionally selected through
the real nested menu with keyboard and pointer input, including the production
host loop and native IME placement. The native fixtures run serially to respect
SDL's event-pump ownership. These application tests do not read back
every backend's pixels; actual-window image checks supplement them. Test executables are staged separately under
`target/neocogi-tests` so they cannot inflate the installed application size. Set
`NEOCOGI_TEST_BUILD_ONLY=1` to stage tests for a separate display-enabled invocation.

The smaller boundary smoke remains available:

```sh
cargo neocogi run --assembly smoke --variant glow -- --smoke
```

It covers typed input/events, inherited and composed controls, compact menus,
native-path file-picker acceptance, invalidation, resize and explicit teardown.
Platform resource tests create real windows and remain explicitly opt-in:

```sh
cargo test -p neocogi-render-plugin --lib --no-default-features --features glow -- --include-ignored
```

On a desktop use its ordinary display. A previous Mesa 25.2.8 offscreen EGL setup
crashed inside `driCreateNewScreen3` when accessible hardware was combined with
`LIBGL_ALWAYS_SOFTWARE=1`, before renderer construction. The ordinary native display
passed; CI uses Xvfb with its available software GPU drivers.

The optional independent extension adds a factory-created counter object
and a custom inset container, with no dependency on the UI or renderer implementation:

```sh
scripts/test-extension.sh glow
scripts/test-extension.sh vulkan
scripts/test-extension.sh wgpu
```

The driver separately builds the extension, host, UI and renderer, and runs from
`/tmp`. The host asks its typed factory to create the counter with `IWidgetHost`.
It verifies common interface identity, shared activation delivery, callback
measurement, unchanged measurement caches, setter/constraint/resize invalidation,
exclusive attachment, topology changes and retained-handle closure. Ordinary demo
packages omit this assembly. See [its source and contracts](../../examples/ui-extension).

## Optional construction workflows

The default file package exposes both construction boundaries. The smoke executable
can derive a larger atlas through GFX or load a JSON theme through skin:

```sh
cargo neocogi run --assembly smoke --variant glow -- --smoke --rebuild-assets
cargo neocogi run --assembly smoke --variant glow -- --smoke --theme themes/windows-95/theme.json
```

Relative theme paths resolve against the executable directory. Use
`cargo neocogi build --assembly full --variant glow --variant embedded` for an explicit embedded deployment;
its GFX and skin plugins expose PNG decoding and live style edits without runtime
atlas building or JSON loading. See
[COM_ASSETS.md](../contracts/COM_ASSETS.md) for immutable catalogs, bounded transfers, upload
atomicity and decoder ownership.

## Browser

```sh
scripts/web-demo.sh serve
scripts/test-browser-input.sh input
scripts/test-browser-input.sh full
```

The browser statically registers the same UI, GFX compositor, platform, and selected
renderer descriptors with one neocogi-com runtime in the wasm module. DOM listeners,
IME, canvas and WebGL2
ownership stay in the selected renderer plugin. The host owns only application state and
one `requestAnimationFrame` scheduling adapter. Every platform window appends its
own canvas; page CSS controls its logical dimensions, and input/frame coordinates
use physical pixels. Do not create an extra placeholder canvas in the page.

`web-demo.sh` honors `CARGO_TARGET_DIR`, `NEOCOGI_WASM_BINDGEN` and
`NEOCOGI_WEB_PROFILE=debug|release` (release by default). It installs the matching
binding CLI when needed and does not require ripgrep. The full demo also statically registers
Skin's live appearance compiler for its style editor, independently of embedded theme tables;
the browser does not need Skin's external JSON loader. Startup errors are reported to the browser
console, and the page retains a visible failure status if no live canvas was published.
A `?frames=8` query runs eight frames and stops, using the
same finite-run policy as native. Normal stop and asynchronous errors release the
RAF closure, application session, canvas, input textarea and listeners. GPU
capabilities retained outside that application still own their device until their
last release.

The input suite checks actual DOM composition/paste, pointer chords, wheel scaling,
static registration, inherited editing, IME placement and WebGL resize. The full
suite exercises theme/image/action behavior, observes real framebuffer color diversity, and verifies both normal and failed RAF
cleanup in Chromium. Set `CHROME_BIN` and `NEOCOGI_BROWSER_TEST_PORT` where needed.

A lost WebGL context reports a renderer error. Applications must recreate the
platform window and its GPU resources; old image/scene capabilities cannot migrate
to a replacement device. Browser file browsing is disabled because this demo's
native-path picker requires an OS filesystem. The same text editing, layout, menus,
style controls and custom rendering remain available.
