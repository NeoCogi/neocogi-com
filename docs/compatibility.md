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
recursive-interface, exclusive/shared lifetime and persistence regressions against
its explicitly built shared runtime. Their names and nonempty execution are
checked; merely installing nightly or finding a successfully compiled binary is
never a passing compatibility case.

Stable/nightly are rolling selectors, not an advertised historical MSRV range.
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
2. Build the runtime explicitly as a `cdylib`. The runtime manifest remains an
   `rlib` by default, preserving the removal of redundant deployment links.
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
canonical weak-origin identity, persistence through provider-owned byte buffers,
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
`checked_values`, `exclusive_com` and `weak_com` targets, with two explicit native-only
exceptions: `buffer_count_includes_each_nested_record` and
`buffers_count_selected_variants_without_charging_larger_alternatives`. These
million-value budget stress cases run in linked, ASan and TSan suites; Miri runs
the remaining 13 checked-value tests, including malformed boolean/enum storage.
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
ThreadSanitizer tests could pass before plugin activation failed on an undefined
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
