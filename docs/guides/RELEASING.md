# Publishing SDK and host implementation releases

This is the maintainer procedure. The [first-use walkthrough](../../examples/getting-started/README.md)
is the user entry point. Publication, Git pushes/tags and account changes are
separate release actions; preparation and local dry-runs upload nothing.

## Compatible host-only patches

An implementation-only host patch may advance `neocogi-com-host` independently
when it preserves the public C ABI, NCB bytes and the published SDK identities.
Keep its SDK/core requirements exact at the existing coordinated version; do not
bump the SDK merely to release an implementation fix. This exception does not
permit changing shared contracts or mixing incompatible SDK generations. All
other public packages remain on the coordinated SDK version. The authoritative
host selection is the exact workspace dependency requirement, checked against
the host's own package version by Cargo and `cargo xtask workspace`; archive
qualification uses that version for the host and the SDK version for its peers.

The prepared host patch is `0.2.1`, with SDK/core/serialization `0.2.0`. The UI
should select `neocogi-com-host = "=0.2.1"` and retain its SDK and serialization
requirements at `=0.2.0`, alongside published platform `=0.3.0-alpha.1`. Qualify
an extracted host archive against the actual registry SDK with stable and Rust
1.85, and the combined published platform consumer with current stable. Inspect
Cargo metadata to reject duplicate SDK identities. Published platform contracts
`0.3.0-alpha.1` use `slice::as_chunks` and do not compile on Rust 1.85; distinguish
that existing platform floor limitation from the host/SDK's supported floor.
Applications must explicitly supply their static or dynamic serialization
provider before persistence. Update the host version, its exact workspace and
tutorial selections, affected lockfiles, changelog and contract documentation.

Run the affected host/persistence tests (including a real deployment DLL),
consumer checks, quality and archive gates and normal nightly min-size build.
The native Linux/macOS/Windows release requirements below still apply; record
actual results rather than inheriting qualification from the prototype. UI
performance must be rechecked against the final published build: the prototype's
Vulkan 2% gate passed, but one CPU benchmark case regressed 3.51%.

From the clean verified candidate, dry-run and publish **only** the host:

```sh
cargo publish --dry-run --locked --registry crates-io -p neocogi-com-host
# Separate authorized release action, after qualification:
cargo publish --locked --registry crates-io -p neocogi-com-host
```

Retain the host archive, SHA-256, exact source commit and dry-run/consumer logs
outside Cargo targets. Use the annotated tag `host-v0.2.1` at that commit, and
push it only after verifying the uploaded archive and registry consumer. Do not
republish existing `0.2.0` packages or retag their coordinated release. Preparation
does not claim a successful upload or completion of remote native qualification.

## Prepare the source before choosing a version

Finish implementation, documentation, examples and compatibility changes first.
Keep the eight SDK packages on one version and their internal dependency requirements
exact for a coordinated release; the compatible host-only exception above is the
only independent implementation version. The authoritative package inventory is `PUBLISHED` in
[packages.rs](../../tools/xtask/src/packages.rs).

Check all eight names and the intended version on crates.io. A name available
during preparation can be claimed before publication; check again at release.
For the first upload, use a crates.io account with a verified email and a token
authorized to publish these names. Configure it interactively with `cargo login`;
keep tokens out of command arguments, logs, source and chat. Confirm ownership
before subsequent uploads. Add any additional owners deliberately after the first
publication. [Trusted publishing](https://crates.io/docs/trusted-publishing) can
replace stored CI tokens for later releases.

The compatibility rule is separate from Cargo versions. The prepared candidate
is `0.2.0`, using ABI v2: runtime and descriptor family 2, core generation
19, and `nc_get_plugin_descriptor_v2`. Deploy hosts, providers and generated
bindings from one SDK release, allowing its compatible host implementation patch.
ABI v1 libraries are rejected before bootstrap;
there is no compatibility export. Interface signatures, IDL encoding and NCB
persistence format have their own authorities in the [ABI contract](../abi-v2.md).


Prepare a user-facing changelog entry with features, migration instructions,
known limits and actual platform results. Keep superseded development entries
historical. The currently prepared version is a release candidate until uploaded.

## Apply the public version last

After all other preparation, update these together:

- `[workspace.package].version` and the six exact internal SDK dependency requirements.
- Root and affected independent fixture lockfiles, preserving unrelated external
  versions and checksums. `cargo xtask workspace` resolves every maintained graph.
- SDK versions in tutorial manifests, installation examples, public README
  snippets and release-tag links. Keep fixture/application versions independent.
- The changelog candidate heading and date. Record the actual publication date
  when releasing; any source edit creates a new candidate to verify.

Commit the complete candidate on the intended release branch. Record its commit
ID with `git rev-parse HEAD`; require a clean index and working tree. Keep plans,
reviews, logs, receipts and copied release archives outside the checkout. A tag
must point to this exact verified commit, never an earlier pre-version revision.

## Qualify the exact candidate

Install stable with rustfmt, Clippy and rust-src, Rust 1.85.0, and the nightly
components required by [repository tasks](../../tools/xtask/README.md). Native
checks require a C compiler and linker. Cargo uses its normal build parallelism;
run compiler-spawning test cases serially. From the clean candidate, run serially:

```sh
cargo fetch --locked
cargo test -p xtask --locked
cargo xtask workspace
cargo xtask quality
cargo test --workspace --all-features --locked
cargo test -p neocogi-com -p neocogi-com-host --lib --release --locked
cargo xtask msrv
cargo xtask packages
cargo xtask compatibility --suite linked
cargo xtask compatibility --suite build-flags
cargo xtask compatibility --suite safety
cargo xtask compatibility --suite address
cargo xtask compatibility --suite thread
```

Require successful native Linux, macOS and Windows jobs for this commit in the
[workflow](../../.github/workflows/rust.yml). They execute the standalone tutorial
against actual SDK archives as well as their native compatibility cases. Linux
safety and build-policy results remain required. Record the actual OS, CPU,
compiler versions and job results; a workflow definition or cross-check is not
native execution. Do not claim 32-bit support without running `cargo xtask abi-32`
with the target and matching C multilib toolchain. Other platforms remain
unqualified unless their workload has actually run.

The archive gate tests the shipped targets, named imported-provider and independent
SharedSources COM consumer cases, docs,
reduced features, generated consumers and independently built tutorial. It uses
only extracted SDKs. It is distinct from the normal registry dry-run below.

Run the ordinary dry-run **online**, with `CARGO_NET_OFFLINE` unset, on a clean
candidate and without `--no-verify` or `--allow-dirty`:

```sh
cargo publish --dry-run --locked --registry crates-io \
  -p neocogi-com-core -p neocogi-idl -p neocogi-com-macros \
  -p neocogi-com -p neocogi-com-serialization -p neocogi-com-serialization-dylib \
  -p neocogi-com-host -p neocogi-inspect
```

Cargo 1.97.1/1.98.1 can fail offline multi-package verification with an internal
`no hash listed` error when a staged dependency has an external dev-dependency.
Normal online verification succeeds for that reproduction and these SDKs. Do
not waive ordinary verification or interpret that offline error as proof of
malformed source. The offline archive gate independently builds and tests its
extracted contents.

Save full logs and all eight `.crate` files from `target/package` outside Cargo
targets, together with their SHA-256 hashes and the candidate commit ID. On Linux
use `sha256sum`, on macOS `shasum -a 256`, or PowerShell `Get-FileHash`. Check the
eight expected versions and `.cargo_vcs_info.json` provenance. Preserve this record
through `cargo clean`. If code, docs, versions or dependencies change, commit a
new candidate and rerun affected checks; refresh packaging/provenance evidence.

## Execute the release

Merge and push the verified commit through the project's normal Git workflow.
Create the matching annotated `vVERSION` tag locally at that exact commit. Keep
the tag local until every package below is published and its registry contents
are verified. Check release-tag link targets locally before uploading; their
remote links become available after the tag is pushed.

Publish only the eight public packages, in dependency order:

```sh
cargo publish --locked --registry crates-io -p neocogi-com-core
cargo publish --locked --registry crates-io -p neocogi-idl
cargo publish --locked --registry crates-io -p neocogi-com-macros
cargo publish --locked --registry crates-io -p neocogi-com
cargo publish --locked --registry crates-io -p neocogi-com-serialization
cargo publish --locked --registry crates-io -p neocogi-com-serialization-dylib
cargo publish --locked --registry crates-io -p neocogi-com-host
cargo publish --locked --registry crates-io -p neocogi-inspect
```

Wait for each dependency to be visible before proceeding. Do not publish xtask,
the integration owners or example applications. Inspect every exit status:
an index-poll timeout can occur after a successful upload. Before retrying, check
which exact versions exist and compare their downloadable archive hashes and
source provenance with the saved candidate. Resume only missing packages when
uploaded content matches. Never silently skip a mismatch or attempt to overwrite
an existing version. Changed public content requires a new coordinated version,
with an explicit follow-up/yank decision for unusable earlier versions.

After all eight packages are published and verified, push the annotated tag. Check
that all rendered release documentation links resolve to that tag and its intended
files. Do not push the tag after only a partial publication.

## Verify registry users' experience

Check docs.rs for all library packages. Copy the complete tagged tutorial into a
fresh directory outside any checkout, and use a fresh task-owned `CARGO_HOME`
without local patches or source replacements. Rebuild and run the documented
host/plugin commands, inspect `cargo metadata` to confirm SDK dependencies came
from the registry, and check the output and overflow assertion. Repeat on Rust
1.85 to check fresh dependency resolution; SDK library lockfiles do not control
a consumer's dependency selection.

Install the explicit released inspector version with `--locked` into a fresh
installation root. Run its `dump` and `check` commands against the tutorial
plugin. A successful upload alone does not complete the release: record any
docs, installation, runtime or platform failure and its follow-up action.

Reference the current [Cargo publication procedure](https://doc.rust-lang.org/cargo/reference/publishing.html)
and [publish command](https://doc.rust-lang.org/cargo/commands/cargo-publish.html)
when carrying out the release. This document does not claim that remote CI,
account checks or post-publication installation have already succeeded.
