# ABI development status

SDK `0.5.0-wip-alpha.1` uses **ABI v4, WORK IN PROGRESS (WIP)**. The ABI is
provisional and remains open to change. Read [the current ABI specification](docs/abi-v4.md)
before changing ABI-facing code. Its version table and the core constants
identify the contract used by this SDK.

Further incompatible design changes can remain part of ABI v4 while it is WIP.
Keep core generation, discovery and admission checks coherent so incompatible
binaries are rejected, and rebuild all participants from the same source revision.
Only describe ABI v4 as final or frozen after an explicit release freeze.

Published SDK `0.4.0` uses ABI v3/core generation 21. Published SDK `0.3.0`
uses ABI v2/core generation 19. Keep their released contracts distinct from
the evolving v4 candidate.

Review reports remain local and ignored; never commit them or put them in Git history.

# Lifetime ownership contract

Ownership policy is established by construction and the declared ABI input/result
contract. Shared raw weak inputs point to NcSharedLifetimeState; boxed ABI inputs
require boxed backing. Provider metadata has no ownership tag. Do not reintroduce
raw expected-kind checks or mixed-policy rejection/cleanup dispatch: a wrong-policy
input violates its producer contract. Keep shape/interface checks, reference bounds,
terminal expiration and actual reader/writer admission. ABI v4/core 22 retains
the four-word type information introduced in ABI v3/core 21.
