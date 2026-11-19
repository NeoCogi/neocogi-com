# COM host

The host role belongs to this final executable. It uses `neocogi-com-sdk` as the
shared SDK for calling provider interfaces and implementing observer callbacks.
See [host/caller/provider roles](../../docs/COM.md#hosts-callers-and-providers).

This executable explicitly owns `neocogi-com-host`. `#[com_main]` initializes
that selected host and supplies immutable access; provider acquisition receives
the same services. The provider has no implementation dependency.

This example uses static NCB helpers for its typed state and explicitly registers
that same static serialization provider for host persistence in both object-plugin
linkage modes. Applications selecting a dynamic codec instead load
`neocogi-com-serialization-dylib` before persistence and omit the static codec.

See the [complete dynamic/static walkthrough](../com-plugin/README.md). Pure saved
state facts live in `support/extent_state.rs`; sharing them never links a provider
into the dynamic executable.
