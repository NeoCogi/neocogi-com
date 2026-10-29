# Checkout tooling integration tests

This unpublished package registers the IDL and inspector integration sources in
their original directories. Relative fixture includes stay with their sources;
the package root remains two directories below the checkout, matching the
existing repository-root calculations. It has no production library or binary.

```sh
cargo test -p neocogi-tooling-tests --locked
```

The public IDL and inspector manifests disable automatic integration-test
discovery and exclude those sources from their archives. Their self-contained
library and binary tests still ship and run under their original packages.
`cargo xtask workspace` checks that every top-level integration source is
registered here and that neither public package advertises it. Compiler-floor
qualification selects the moved IDL semantic cases through this owner.

The inspector suite builds the real CLI and obtains its executable from Cargo's
artifact output; it cannot use another package's `CARGO_BIN_EXE` variable.
