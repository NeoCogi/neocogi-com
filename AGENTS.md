# ABI development status

SDK `0.5.0-wip-alpha.1` uses **ABI v5, WORK IN PROGRESS (WIP)**. Read
[the current ABI specification](docs/abi.md) before changing ABI-facing code.

`NC_COM_ABI_VERSION` is the shared native compatibility version. Advance it
for incompatible changes and rebuild hosts, providers, and generated consumers
together. Keep the descriptor getter signature and the fixed marker/version
header readable across versions so old layouts can be rejected before use. Only
describe the ABI as frozen after an explicit release freeze.

Published SDK `0.4.0` uses ABI v3/core generation 21. Published SDK `0.3.0`
uses ABI v2/core generation 19. Use their release-tag specifications.

Review reports remain local and ignored; never commit them or put them in Git history.

# Lifetime ownership contract

Ownership policy is established by construction and the declared ABI input/result
contract. Shared raw weak inputs point to NcSharedLifetimeState; boxed ABI inputs
require boxed backing. Provider metadata has no ownership tag. Do not reintroduce
raw expected-kind checks or mixed-policy rejection/cleanup dispatch: a wrong-policy
input violates its producer contract. Keep shape/interface checks, reference bounds,
terminal expiration and actual reader/writer admission. ABI v5 retains
the four-word type information introduced in ABI v3/core 21.
