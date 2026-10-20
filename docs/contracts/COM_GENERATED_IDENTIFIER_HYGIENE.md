# Generated COM identifiers

Legal authored names identify public fields, methods, types and descriptor
entries. They must not become implementation-local variables in generated
conversions. A growing blacklist of names such as `budget` or `raw_0` cannot
establish this contract.

## Contract ownership

IDL is the only callable/value declaration frontend. Its resolver binds each
reference once, retains the supplying Rust projection separately from canonical
identity, and passes typed ownership plans to `neocogi-com-codegen`. The backend
emits complete Rust. It never parses generated Rust wrappers to rediscover a
contract and accepts no optional author-assigned identity/signature modes.

`com_object` and `plugin` still accept ordinary Rust implementation syntax.
`ComSerialize` and `ComDeserialize` remain standalone persistence derives. Their
Rust input may contain private names, const expressions, application `Self` and
opaque type macros that IDL deliberately does not admit. Their lexical and
support-lookup obligations remain independently tested.

## Three naming rules

1. Support operations name their actual trait, type, variant or macro owner.
2. Private implementation definitions live in private lexical or trait scopes.
3. Deliberate public generated names share explicit admission and emission policy.

A private prefix or mixed-site span alone satisfies none of these rules. Printed
Rust loses procedural syntax context, and ambient constants can capture a fresh
pattern spelling. All generated output must preserve the same behavior after
printing and reparsing.

## Values, methods and native fields

Authored field/parameter names remain only in application members, trait
signatures and descriptions. Conversion bodies use separate positional bindings.
For example, `first: bool` and `raw_0: u8` cannot accidentally encode the converted
boolean into the second field. Actual ABI calls assert independent sentinel
values, including copied/owning records, enums, nested arrays and buffers.

`PrivateNames` owns each emitting scope. It allocates item/type names with
`fresh`, executable names with `binding`, and buffer/owner suffix families
atomically. Seed it with all supplied syntax before introducing private names.
Anonymous implementation scopes import `::core::convert::identity` under the
registered pattern names. These never-called imports prevent ambient constants
from changing a pattern into a constant match after printing. They are required
lexical protection, not runtime state.

Wire, payload and backing-storage declarations live in anonymous scopes. The
public raw integration boundary is `<Value as ComValue>::Wire` and `::Storage`;
private helper spellings are not APIs. Converters explicitly select `ComValue`
operations and constants, even when a native type has same-named inherent members
with plausible wrong values. Layout identity additionally proves recursive size,
alignment, field offsets and absence of content checks or ownership.

Native field types resolve at their application owner before private decoder
scopes exist. Private lexical projections preserve `Self`, recursive `Vec<Self>`,
const expressions and opaque macros for standalone persistence derives without
requiring a live COM value contract. Public native records may retain private
field types. No erased registry or public field-resolution API is introduced.

## Public method and companion admission

Core owns `NC_COM_HANDLE_METHOD_NAMES` and `is_reserved_com_method`. IDL rejects
callable names shadowed by actual handle operations and physical vtable prefixes.
Parameters with those spellings remain legal. A source audit compares the real
public handle receiver methods, including helper traits, to this inventory.
An accepted provider method cannot silently dispatch to a same-typed handle
helper such as `downgrade`; use a name such as `weak_capability`.

Interface companions and dense-enum public members also share core inventories
between admission and emission. Private implementation names reserve no source
namespace. Provider/client binders are allocated against the supplied typed plan;
Rust object generics retain their authored names, defaults and where clauses.
Construction takes complete native values; weak wiring follows ordinary
allocation, so there is no cyclic-constructor binder or promotion state.

## Support lookup and plugins

Support types/variants are qualified through `::core` and the explicit COM facade.
`Option`, `Some` and `None` remain legal IDL type names. Generated standard derives,
assertions and other macros name their actual owner. Integer-enum and flag helpers
select `ComValue` operations explicitly and inline the declared flag-bit union
rather than inventing a private inherent `KNOWN_BITS` member.

Plugin acquisition uses `::core::option::Option::Some`. An omitted version uses
`::core::env!`. These are separate namespace obligations: qualifying an Option
variant does not protect a macro invocation. Neither is related to the exact
hexadecimal Cargo-owner encoding in [IDL modules](IDL_MODULES.md).

The `plugin_descriptor()` accessor lexically owns descriptor/dependency statics
and its C acquisition adapter. Allocate their names against acquisition, interface,
GUID and dependency expressions before introducing implementation scope. A fixed
name inside a function could otherwise capture the function's own inputs.
Metadata stays process-lived; the descriptor introduces no second global registry.

The trailing-field default policy invokes
`<Self as ::core::default::Default>::default()`. An inherent `Self::default`, even
with the same return type and a plausible different result, cannot substitute.
Only absent trailing fields default; malformed or truncated present data fails.

## Regression responsibilities

The backend's executable consumers exercise ordinary and printed object/plugin/
persistence expansions, generated IDL codecs, same-typed wrong-value traps,
wrong-return-type traps, ambient constants and actual C acquisition calls.
Identifier fixtures execute owner rejection/retry, output publication and inherited
client calls. Both the core persistence ABI and facade provider bootstrap are
freshness-checked against this same typed backend; a change must regenerate both.

Rust declaration-attribute fixtures were retired with their frontend. They are
not a supported route around IDL admission. Their supported runtime assertions
now use actual IDL sources; ordinary Rust scope assertions remain on the surviving
implementation/persistence procedures. Run the compiler-owned backend regressions with
`cargo test -p neocogi-idl --lib -- --test-threads=1`.
