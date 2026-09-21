# neocogi-com-events-contracts

This `no_std` contract crate generates the `neocogi.com.events` interfaces from
[its IDL module](idl/neocogi/com/events). It depends on the COM facade's macros;
it does not enable allocation, standard channels, or a linked process runtime.

| Contract | Operation and ownership |
| --- | --- |
| `IEventDispatcher` | `dispatch(&mut self) -> Result<bool>` consumes at most one message. `true` reports work; `false` reports no work. Scheduling and budgets belong to the caller. |
| `INamedOutputs` | `bind_output(&mut self, name: &[u8], receiver: &IUnknown) -> Result<()>` is the IDL signature. Generated Rust uses a readable byte-buffer argument and a borrowed `IUnknown`; a successful binding retains the typed receiver through the producer's setter. |

`EVENT_E_LIMIT` is status `-1007`, used by the retained UI runtime for update
budget exhaustion. `EVENT_E_CLOSED` and `EVENT_E_BUSY` belong to the Rust
adapters in [`neocogi-com-events`](../events/README.md).

Concrete message receivers are generated with each `#[message]` declaration in
its owning module. They are not collected in this crate. Named binding resolves
the receiver's exact interface before replacing an existing binding; the
standard implementation is `NamedOutputs` in the adapter crate.

`build.rs` uses `neocogi-idl-build` to emit Rust bindings and schema-4 source-graph
metadata through Cargo `links`/`DEP_*`. No contract shared library, runtime
registration hook, or event queue is emitted. See [IDL setup](../idl/README.md).
