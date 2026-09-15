# neocogi-com-runtime

`neocogi-com-runtime` implements the process-global state behind
`libnc_com_shared_runtime`. Applications normally use the safe
`neocogi_com::Runtime` facade; assembly tooling links this implementation into
the one deployed runtime shared library.

## Responsibilities

The runtime owns:

- canonical contract-module registration;
- generation and one-time seeding of the intrinsic `neocogi.com.events` description;
- factory-plugin registration and dependency ordering;
- trusted dynamic-library loading;
- typed root-factory activation;
- loaded-module provenance and read-only inspection;
- callback-based diagnostics.

It does not own object constructors or select implementations by class ID.
After one fixed root bootstrap, construction is an ordinary typed method call on
the plugin's generated factory interface.

## Dynamic loading

A dynamic plugin must export only `nc_get_factory_plugin_description`. Loading
proceeds as follows:

```text
map plugin and its fingerprint-qualified contract libraries
  -> read and validate the factory plugin description
  -> traverse ordinary contract getters dependency-first
  -> resolve built-in ID/fingerprint edges against the seeded catalog
  -> reject duplicate IDs, cycles, missing plugins, or fingerprint conflicts
  -> publish the complete graph and retained module atomically
  -> activate the root only when the host requests its exact factory type
```

The first catalog-dependent runtime API call lazily validates and seeds the intrinsic
events description before publishing any catalog state. `OnceLock` serializes
concurrent calls and retains a structural failure as an error; no host startup
step or shared-library loader callback is involved. Hosts must not invoke COM
from loader initialization or a plugin description getter.

Live loading is required and fail-fatal: a path, loader, symbol, description,
dependency, validation, or registration failure is diagnosed and aborts instead
of returning partially visible state. Successful libraries remain mapped until
process exit. Loading must not be nested from loader initialization or a
description getter.

The non-registering inspection operation opens a trusted library, validates its
`NcFactoryPluginDescription`, invokes the callback synchronously, and copies no
borrowed pointers past that callback. Tooling uses this path before staging an
assembly.

## Registry model

Contract traversal happens before the catalog write lock. One logical module ID
may be registered repeatedly only with the same full fingerprint. Plugin
publication includes its complete contract graph, dependency relationships, and
module provenance in one write transaction. Plugin code, factory activation,
native loader hooks, description getters, and diagnostic callbacks are never
invoked while that lock is held.

Runtime ABI v2 exposes separate C tables for operations, plugin enumeration,
contract enumeration, inspection, and diagnostics. Tables begin with a size so
new fields can be appended compatibly within v2. `NcModuleInfoV2` owns process
provenance; `NcPluginInfoV2` owns plugin name/version metadata. The Rust facade
joins them into an owned `LoadedModuleInfo`.

The runtime retains at most 1,024 dynamically loaded modules and permits 64
simultaneous debug messengers. Module-capacity failure follows the fail-fatal
loading path; debug capacity returns `NC_E_OUT_OF_RANGE`.

The runtime is intentionally `std`-only. This does not change the `no_std`
support of generated contracts, `neocogi-com-core`, or reduced-feature
`neocogi-com` consumers.
