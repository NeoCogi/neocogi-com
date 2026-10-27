# COM host

This executable explicitly owns `neocogi-com-host`. `#[com_main]` initializes
that selected host and supplies immutable access; provider acquisition receives
the same services. The provider has no implementation dependency.

See the [complete dynamic/static walkthrough](../com-plugin/README.md). Pure saved
state facts live in `support/extent_state.rs`; sharing them never links a provider
into the dynamic executable.
