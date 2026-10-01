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
