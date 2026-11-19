# Linked host fixture

The application links the host rlib and generates its bindings independently
from providers A and B. Both providers are built as native shared libraries.
The application loads B, then A. A acquires B during bootstrap and again during
method dispatch. B requires A to be visible in the same catalog. Repeated host
initialization and a second thread both see the same two plugin records.

The integration test copies Cargo-reported artifacts into a separate deployment
directory whose path contains spaces and Unicode. Run it from the checkout.

```sh
cargo test -p neocogi-com-tests --features runtime --test linked_host --locked
```
