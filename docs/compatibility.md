# Executed COM compatibility checks

The interoperability contract is a matching target C ABI, current core/runtime
ABI generation, and exact generated contract signatures. Independently selected
Rust compilers, optimization, LTO and panic settings must not leak private Rust
layout or allocator ownership across that boundary. This does not promise that
old contract revisions or different architectures are interchangeable.

[`scripts/compatibility.py`](../scripts/compatibility.py) owns the independent
build and execution policy. It uses installed Rustup toolchains and Python 3.10+
with only the standard library. No wrapper Cargo project or extra production
crate is introduced. Run from any working directory:

```sh
python3 crates/com/scripts/compatibility.py --suite cross-version --report /tmp/com-cross.json
python3 crates/com/scripts/compatibility.py --suite build-flags --report /tmp/com-flags.json
python3 crates/com/scripts/compatibility.py --suite linked --report /tmp/com-linked.json
python3 -m unittest discover -s crates/com/scripts -p test_compatibility.py
```

The matrix is deliberately explicit, rather than a claim that every possible
compiler flag is interchangeable:

| Case | Shared runtime | Host | Plugin |
| --- | --- | --- | --- |
| linked | stable, debug | stable, debug | stable, debug |
| stable-host | stable, debug | stable, debug | nightly, release, abort, fat LTO, randomized private layout |
| nightly-host | nightly, debug | nightly, release, abort, thin LTO, randomized private layout | stable, debug |
| stable-release | stable, release, abort, fat LTO | stable, release, unwind, thin LTO | stable, release, abort, fat LTO |

Unspecified panic/LTO settings are unwind/off. Randomization uses seed 107.
`cross-version` executes the two mixed-compiler rows; `build-flags` adds
`stable-release`. `linked` also discovers and runs current C-layout, checked-value,
recursive-interface, prepared consuming-output completion, exclusive/shared
lifetime and persistence regressions against
its explicitly built shared runtime. Their names and nonempty execution are
checked; merely installing nightly or finding a successfully compiled binary is
never a passing compatibility case.

Stable/nightly are rolling selectors, independent of the COM crates' declared
Rust 1.85 minimum. The separate `com-msrv` CI job runs the existing floor suite:

```sh
rustup toolchain install 1.85.0 --profile minimal
# The workspace lock is untracked; resolve once before the locked suite.
cargo +1.85.0 generate-lockfile
bash crates/com/scripts/test-msrv.sh
```

That suite checks all COM/IDL/inspector targets and features, library tests,
normative ABI snippets, inspector binary tests, and the facade/core feature
variants. It verifies compiler support; it does not execute the cross-compiler
dynamic-loader matrix or promise the same floor for UI/rendering dependencies.

Reports record `rustc -vV` for every selected compiler plus each role's exact
profile, panic and LTO policy. Reports change to `running` before work begins and
`failed` on error; an older success cannot remain as the current result. The
platform must permit native execution with matching host triples. Linux, macOS
and Windows artifact names are handled; MSVC's import library is prepared only
for linking and is not shipped as a runtime dependency. Platform qualification
comes from executing its CI row, not from another platform's successful build.

## What one case does

1. Validate installed compilers and their native target agreement. Remove ambient
   compiler, target, profile and loader-path overrides from the build environment.
2. Build the runtime explicitly as a `cdylib`. Its manifest offers both `rlib`
   and `cdylib`: static consumers select the former; shared hosts use `libnc_com`.
3. Build the plugin and host independently, each with its own Cargo target and
   compiler/profile policy. The host links the freshly built runtime.
4. Require the exact nonempty artifacts. Copy only the executable, runtime and
   plugin into a relocation directory. Clean each build target after copying it.
5. Run the existing [example host](../../../examples/com-plugin/src/main.rs) from
   that directory, with the loader path restricted to the staged libraries and
   the temporary runtime link directory removed. Ordinary OS dependencies remain.
6. Require successful exit and the completion marker printed after all assertions.

The shared example exercises typed factory construction, inherited interface
calls, exclusive mutation, events delivered into the host's concrete queue,
canonical weak-origin identity, fresh copied output (nested enum/boolean and short
prefix) through ABI 17, persistence through provider-owned byte buffers,
restoration through a host-implemented factory adapter, promotion while live,
and failed promotion after final object release. All assertions execute in
release/abort builds. Normal completion crosses calls in both directions; panic
settings are build policies, not permission to unwind over a COM call.

Native C layout and malformed copied/nested boolean values are separate explicit
checks in `linked`, as are the full unique-owner and shared weak-lifetime test
targets. They are not falsely described as coverage provided by the small
cross-compiler example alone. The monolith remains a separate integration test
because it must work without any native NeoCogi libraries.

## Entry points, safety checks and CI

The shell entry points `test-cross-version.sh`, `test-build-flags.sh`, and
`test-linked-runtime.sh` select the corresponding driver suite. The linked
PowerShell entry point selects the same suite and propagates its exit code.
They accept driver arguments such as `--report`; there is no independent shell
matrix or old arbitrary Cargo-argument forwarding to keep in sync.

`test-safety.sh` selects `--suite safety`. It requires nightly with `miri` and
`rust-src` installed. It discovers and interprets the complete COM core, runtime,
and facade library tests, plus current `local_com`, `borrowed_values`,
`checked_values`, `output_buffers`, `output_completion`, `exclusive_com` and
`weak_com` targets.
Four explicit large-buffer stress cases remain native-only: the two checked-value
budget traversals and the output-completion maximum-variant/provider-publication
stress tests. Their exact names live in the driver. They run in linked, ASan and
TSan suites; Miri runs the remaining 13 checked-value tests, six directional output-buffer
tests and eleven output-completion tests, including allocation injection, preserved owners, and malformed outputs.
Facade tests select the static runtime because Miri interprets pure Rust and
cannot load native plugins.
That is a different check from executing the real shared-library boundary.

`test-sanitizers.sh` selects `--suite sanitizers`, executing address and then
thread instrumentation and failing on either error. The `address` and `thread`
suites also run individually. These require nightly `rust-src` and
`x86_64-unknown-linux-gnu`. Each rebuilds the standard library, runtime, host,
plugin and test harnesses under the same sanitizer. Instrumented executables
export sanitizer hooks before linker section collection: a later-loaded plugin
may need a hook that the host itself never called. Without that export,
ThreadSanitizer tests could pass before plugin acquisition failed on an undefined
`__tsan_atomic64_exchange`. The six Rust targets from `linked` and the three
library suites execute against the instrumented runtime;
the relocated example then executes without its build targets. Native-C layout
checks remain in `linked`, since this driver does not instrument the C compiler.
Leak reporting is disabled for sanitizer execution because successful plugin
libraries and process runtime registrations intentionally remain pinned. Address
and data-race checks remain enabled. Miri receives no suppression flags.

```sh
rustup toolchain install nightly --component miri --component rust-src
crates/com/scripts/test-safety.sh --report /tmp/com-miri.json
python3 crates/com/scripts/compatibility.py --suite address --report /tmp/com-asan.json
python3 crates/com/scripts/compatibility.py --suite thread --report /tmp/com-tsan.json
```

Missing components, unsupported sanitizer targets, discovery mismatches and
instrumentation errors fail the invocation; they are never silently skipped.
The combined sanitizer report records both cases and is successful only after
both complete. Temporary targets are deleted even when verification fails;
passing native roles also run explicit `cargo clean` after copying. Miri's task
owns its private temporary target, not the user's shared interpreter sysroot
cache; cleanup leaves that shared compiler cache intact.

[CI](../../../.github/workflows/rust.yml) keeps ordinary static/unit/IDL tests
separate from the independent-build matrix. Linux executes the three mixed-build
rows; Linux, macOS and Windows each execute the linked row. Separate Linux jobs
run Miri and both sanitizers. A platform job is not evidence until it actually
runs successfully; local Linux validation does not imply macOS/Windows execution.

The ordinary COM job also runs facade/runtime library tests in release mode.
Strong-only retain exhaustion formerly returned uncounted owners only when debug
assertions were disabled, so debug tests alone cannot prevent its recurrence.
Both profiles exercise injected counter limits and native subprocess termination
through byte/string/list `Clone` and `QueryInterface` entrypoints. Miri skips those
process-abort cases and interprets the ordinary count/final-destruction tests,
including child-list deallocation through its original owning pointer.

## Why the old entry points failed

`8fed79b2` made the runtime an rlib to avoid an unused duplicate shared-library
link. `5212532c` removed the obsolete cross-version component fixture during
factory-plugin migration. `12f49a87` removed native contract DSOs and renamed the
dynamic-plugin test. Some scripts and CI retained the previous artifact command
or test name. Discovery guards in linked/build-flags scripts correctly rejected
stale names; a silent zero-test pass was a general filtering risk, not what those
guarded scripts did.

The current driver retains the valid architecture and replaces duplicate dynamic
fixture builders. Ordinary generated factory registration still has its own
integration test. Neither the old component fixtures nor native contract
libraries need to be restored to repair this verification boundary.

## Validation recorded on 2026-09-30

The implementation is in `338e2963` (canonical driver/fixture) and `e69f2ecc`
(script/CI/safety migration), on branch `fix/com-independent-build-checks`.
Fresh native execution used `x86_64-unknown-linux-gnu`, stable
`rustc 1.98.1 (48a229cea 2026-09-01)` and nightly
`rustc 1.101.0-nightly (c1070d693 2026-09-28)`.

| Check | Executed result |
| --- | --- |
| `build-flags` | All three rows passed, including stable/nightly in both directions and the mixed stable release row |
| `test-linked-runtime.sh` | 57 tests passed, including 3 native-C tests; relocated host/plugin execution passed |
| `test-safety.sh` | 154 Miri tests passed; task-owned target cleanup completed |
| `address` | 161 tests passed; relocated host/plugin execution passed |
| `thread` | 161 tests passed; relocated host/plugin execution passed after exporting sanitizer hooks |
| Repaired static COM/core/macros/runtime/inspector CI command | 267 tests/doctests passed, including generated compile cases |
| Example `build_modes` | Relocated monolith passed without native NeoCogi libraries |
| Driver regressions | 9 passed, including partial sanitizer results and a failed process printing a success marker |
| Entry-point/doc checks | Bash syntax, workflow YAML parsing and COM Markdown links passed |

These are Linux results. The macOS/Windows matrix and PowerShell wrapper require
their respective CI execution; their presence in a workflow is not a passing
result. The large budget tests omitted only from Miri passed in the linked and
both sanitizer runs. No runtime ABI change or historical contract-DSO restoration
was needed to resolve the verification gap.

## Output completion coverage added on 2026-10-01

The prepared-output fix (`d586d06f`, `1ca86a32`) adds `output_completion` to
linked/sanitizer targets and the Miri target list. Fresh linked execution passed
72 tests, including its 15 new regressions, and the relocated host/plugin check.
A focused Miri run passed its 13 small regressions; two large-buffer cases ran
natively. The driver unit suite passed all 9 tests. Full safety, sanitizer, and
mixed-compiler suites were not rerun for this addition. The prior dated matrix
above remains historical evidence, not a fresh result for these new tests.
See the [completion contract and validation record](../../../docs/contracts/COM_OUTPUT_COMPLETION.md)
for allocation injection, output bounds, owner preservation, and consumer checks.

## Published archive validation

Run `bash crates/com/scripts/test-packages.sh` with Rust 1.85 installed. It creates
real Cargo archives for the five SDK packages and inspector, extracts them into a
fresh directory, and patches dependencies **only to those extracted archives**.
Rust 1.85 checks all shipped targets and runs shipped executable tests and
runnable documentation examples. An ordinary runtime build must produce both its
rlib and platform shared library, matching the public build guide. One temporary shared target
cache avoids retaining one copy per package and is removed after verification. The gate also builds a fresh consumer
with `build.rs`, generates an interface and executes its checked COM call. Runtime
and descriptor discovery are asserted to remain v1. No-default and alloc-only
facade checks cover the portable feature boundary. CI runs this same gate.

IDL, facade and inspector repository integration suites inspect sibling packages,
current guide examples or nested native fixture workspaces. Their source files
stay in checkout CI and are excluded from registry archives, so Cargo does not
advertise test targets whose inputs are absent. Package-local unit tests remain
included. Macro printer fixtures belong to the macro archive; the core handle
source audit belongs to core. The small subprocess helper is deliberately local
to each owning package rather than requiring a published test-support crate.

The macro package's executable tests create an ordinary matching-version facade
consumer. Cargo metadata supplies the selected core/facade source locations;
there is no `../core` or `../api` convention. In a checkout it reuses Cargo's
workspace package. Archive validation supplies all SDK archives through normal
patch configuration; after publication the same consumer resolves the published
version. These checks require that matching SDK and its registry dependencies
be available offline. They do not introduce a production macro-to-facade dependency
or a cyclic publication order.
