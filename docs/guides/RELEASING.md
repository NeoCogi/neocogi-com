# Publishing a coordinated SDK release

This is the maintainer procedure. The [first-use walkthrough](../../examples/getting-started/README.md)
is the user entry point. Publication, Git pushes/tags and account changes are
separate release actions; preparation and local dry-runs upload nothing.

## Prepare the source before choosing a version

Finish implementation, documentation, examples and compatibility changes first.
Keep the six SDK packages on one version and their five internal requirements
exact. The authoritative package inventory is `PUBLISHED` in
[packages.rs](../../tools/xtask/src/packages.rs).

Check all six names and the intended version on crates.io. A name available
during preparation can be claimed before publication; check again at release.
For the first upload, use a crates.io account with a verified email and a token
authorized to publish these names. Configure it interactively with `cargo login`;
keep tokens out of command arguments, logs, source and chat. Confirm ownership
before subsequent uploads. Add any additional owners deliberately after the first
publication. [Trusted publishing](https://crates.io/docs/trusted-publishing) can
replace stored CI tokens for later releases.

The compatibility rule is separate from Cargo versions. Deploy hosts, providers
and generated bindings from one SDK release. Core generation 18 freezes the
first public layouts and callback signatures; an incompatible change needs a
fresh applicable generation before interpreting a full record or invoking code.
Runtime and descriptor discovery retain family 1. Interface signatures, IDL
encoding and NCB persistence format have their own authorities in the
[ABI contract](../abi-v1.md).

Prepare a user-facing changelog entry with features, migration instructions,
known limits and actual platform results. Keep superseded development entries
historical. The currently prepared alpha is a release candidate until uploaded.

## Apply the public version last

After all other preparation, update these together:

- `[workspace.package].version` and the five exact SDK dependency requirements.
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
checks require a C compiler and linker. Keep compiler and test concurrency at
the repository's existing bound. From the clean candidate, run serially:

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
  -p neocogi-com -p neocogi-com-host -p neocogi-inspect
```

Cargo 1.97.1/1.98.1 can fail offline multi-package verification with an internal
`no hash listed` error when a staged dependency has an external dev-dependency.
Normal online verification succeeds for that reproduction and these SDKs. Do
not waive ordinary verification or interpret that offline error as proof of
malformed source. The offline archive gate independently builds and tests its
extracted contents.

Save full logs and all six `.crate` files from `target/package` outside Cargo
targets, together with their SHA-256 hashes and the candidate commit ID. On Linux
use `sha256sum`, on macOS `shasum -a 256`, or PowerShell `Get-FileHash`. Check the
six expected versions and `.cargo_vcs_info.json` provenance. Preserve this record
through `cargo clean`. If code, docs, versions or dependencies change, commit a
new candidate and rerun affected checks; refresh packaging/provenance evidence.

## Execute the release

Create the matching annotated `vVERSION` tag at the verified commit, and publish
the branch/tag through the project's normal Git workflow. Check that all rendered
release documentation links now resolve to that tag and its intended files.
This remote check happens after tag creation, not before version preparation.

Publish only the six public packages, in dependency order:

```sh
cargo publish --locked --registry crates-io -p neocogi-com-core
cargo publish --locked --registry crates-io -p neocogi-idl
cargo publish --locked --registry crates-io -p neocogi-com-macros
cargo publish --locked --registry crates-io -p neocogi-com
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

## Verify registry users' experience

Check docs.rs for all library packages. Copy the complete tagged tutorial into a
fresh directory outside any checkout, and use a fresh task-owned `CARGO_HOME`
without local patches or source replacements. Rebuild and run the documented
host/plugin commands, inspect `cargo metadata` to confirm SDK dependencies came
from the registry, and check the output and overflow assertion. Repeat on Rust
1.85 to check fresh dependency resolution; SDK library lockfiles do not control
a consumer's dependency selection.

Install the explicit alpha inspector version with `--locked` into a fresh
installation root. Run its `dump` and `check` commands against the tutorial
plugin. A successful upload alone does not complete the release: record any
docs, installation, runtime or platform failure and its follow-up action.

Reference the current [Cargo publication procedure](https://doc.rust-lang.org/cargo/reference/publishing.html)
and [publish command](https://doc.rust-lang.org/cargo/commands/cargo-publish.html)
when carrying out the release. This document does not claim that remote CI,
account checks or post-publication installation have already succeeded.
