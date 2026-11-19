# Publishing a coordinated SDK release

This is the maintainer procedure. The [released first-use walkthrough](https://github.com/NeoCogi/neocogi-com/blob/v0.3.0/examples/getting-started/README.md)
is the user entry point. Publication, Git pushes/tags and account changes are
separate release actions; preparation and local dry-runs upload nothing.

## Prepare the source before choosing a version

Finish implementation, documentation, examples and compatibility changes first.
Keep the nine SDK packages on one version and their internal dependency requirements
exact. The authoritative package inventory is `PUBLISHED` in
[packages.rs](../../tools/xtask/src/packages.rs).

The ninth package, `neocogi-com-host-dylib`, is new for the
`0.4.0-alpha.1` candidate. The published `0.3.0` cohort consists of eight packages.
Preparing this branch does not change the published version or release tag.

Check all nine names and the intended version on crates.io. A name available
during preparation can be claimed before publication; check again at release.
For the first upload, use a crates.io account with a verified email and a token
authorized to publish these names. Configure it interactively with `cargo login`;
keep tokens out of command arguments, logs, source and chat. Confirm ownership
before subsequent uploads. Add any additional owners deliberately after the first
publication. [Trusted publishing](https://crates.io/docs/trusted-publishing) can
replace stored CI tokens for later releases.

The compatibility rule is separate from Cargo versions. The current published
release is `0.3.0`, using ABI v2: runtime and descriptor family 2, core generation
19, and `nc_get_plugin_descriptor_v2`. The unreleased development branch uses
ABI v3: runtime and descriptor family 3, core generation 21, and
`nc_get_plugin_descriptor_v3`. Deploy hosts, providers and generated bindings
from one SDK release. Development hosts reject ABI v1/v2 libraries before
bootstrap; there is no compatibility export. Interface signatures, IDL encoding
and NCB persistence format have their own authorities in the
[development ABI contract](../abi-v3.md).


Prepare a user-facing changelog entry with features, migration instructions,
known limits and actual platform results. Keep superseded development entries
historical. For future releases, a prepared version remains a candidate until
its publication and registry-user checks are complete.

COM `0.3.0` was published on 2026-11-15 from `35db184` after its
[required CI jobs passed](https://github.com/NeoCogi/neocogi-com/actions/runs/43675768485).
The `v0.3.0` tag now points to `57e913f`, which updates release wording and
retains the same tutorial and SDK code.
All eight registry archives matched the verified candidate's hashes and clean
Git provenance. Registry-only tutorial builds and real plugin calls passed on
stable Rust and Rust 1.85; the registry-installed inspector checked both plugins,
and docs.rs documentation is available for all seven library crates.

## Persistence migration for 0.3.0

All eight COM packages were released together. The unpublished host-only `0.2.1`
candidate is superseded; no host-specific version policy or archive handling
remains. Applications explicitly register the static serialization descriptor or
load the serialization dylib before persistence. Missing providers return
`NC_E_NOT_FOUND`; persistence always requests named NCB.

This release does not promise Rust dependency compatibility with the published
`0.2.0` SDK. Move downstream UI and platform compositions to the new COM cohort
together; do not combine COM Rust types from different package versions. ABI
family 2, core generation 19, IDL encoding v14 and NCB-v1 bytes retain their
independent authorities in that published release. The development lifetime split
uses the [current discovery and core versions](../abi-v3.md#abi-identities-and-versions)
without changing IDL v14 or NCB-v1.

Merge the candidate to master and push for native CI before publication. Publish
only after every required job for that exact commit passes. Historical prototype
results do not qualify its performance: Vulkan passed its 2% gate, but one UI CPU
case regressed 3.51% and must be rechecked against the final published build.

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

Run the ordinary dry run **online** with the release toolchain from the clean
candidate checkout. Unset `CARGO_NET_OFFLINE` and keep Cargo verification enabled.
`--workspace` selects publishable members and skips `publish = false` members.
The `cargo xtask workspace` check above compares that set with the archive
inventory. Multi-package publishing requires Cargo 1.90 or later.

```sh
cargo publish --dry-run --locked --registry crates-io --workspace
```

Cargo 1.97.1/1.98.1 can fail offline multi-package verification with an internal
`no hash listed` error when a staged dependency has an external dev-dependency.
Normal online verification succeeds for that reproduction and these SDKs. Do
not waive ordinary verification or interpret that offline error as proof of
malformed source. The offline archive gate independently builds and tests its
extracted contents.

Save the online dry-run log with the exact candidate commit ID. Cargo 1.93 and
later treats publish tarballs as intermediate artifacts. To retain `.crate`
files, run ordinary `cargo package --locked --registry crates-io` on the same
clean Git commit with one `-p` selector for each of the nine entries in the
[SDK inventory](../../tools/xtask/src/packages.rs). Cargo package selection can
include unpublished members, so use the validated publication set here. The
extraction gate tests the archive contents using temporary source copies. The
saved release archives establish clean Git provenance.

Save all nine resulting archives outside Cargo targets and record their SHA-256
hashes. Use `sha256sum` on Linux, `shasum -a 256` on macOS or `Get-FileHash` on
Windows. Check the versions and `.cargo_vcs_info.json` against the candidate.
Preserve this evidence through `cargo clean`. If source, documentation, versions
or dependencies change, verify a new candidate and refresh affected evidence.

## Execute the release

Merge and push the verified commit through the project's normal Git workflow.
Create the matching annotated `vVERSION` source tag at that exact commit and
push it with master as authorized for this release. A pushed source tag does not
claim crate publication. Check release-tag link targets locally before pushing.
Do not publish any package until every required CI job passes for this exact
tagged commit; a previous alpha's result or a different master revision does not
satisfy the gate.

Publish only the nine public packages, in dependency order:

```sh
cargo publish --locked --registry crates-io -p neocogi-com-core
cargo publish --locked --registry crates-io -p neocogi-idl
cargo publish --locked --registry crates-io -p neocogi-com-macros
cargo publish --locked --registry crates-io -p neocogi-com
cargo publish --locked --registry crates-io -p neocogi-com-serialization
cargo publish --locked --registry crates-io -p neocogi-com-serialization-dylib
cargo publish --locked --registry crates-io -p neocogi-com-host
cargo publish --locked --registry crates-io -p neocogi-com-host-dylib
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

After all nine packages are published and verified, record the registry archive
hashes and provenance alongside the already pushed source tag. Check that all
rendered release documentation links resolve to that tag and its intended files.
A partial or failed publication must be reported separately from the source tag;
never move an existing release tag to conceal a changed or incomplete candidate.

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
