# Shared host fixture

The application generates its bindings independently from providers A and B.
Its default graph includes the SDK and compiler but no host implementation.
It loads the real host shim, then B and A; A acquires B during bootstrap and
again during method dispatch. B requires A to be visible in the same catalog.
Repeated runtime bootstrap observes those same two records after its native
wrapper has left scope, including from a second thread.

The integration owner copies Cargo-reported native artifacts into a separate
Unicode/space-containing deployment directory. A second application build with
`static-host` selects the rlib instead and runs the same providers/assertions in
a fresh process. Missing image/export and real failing/null/incompatible native
bootstraps exercise startup errors without any static host fallback.

Run from the checkout:

```sh
cargo test -p neocogi-com-tests --features runtime --test shared_host --locked
```
