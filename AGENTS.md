# ABI development status

ABI v3 is **WORK IN PROGRESS (WIP)** for the `0.4.0-alpha.1` candidate. It is
provisional and not frozen. Read [the current ABI specification](docs/abi-v3.md)
before changing ABI-facing code; its version table and the core constants
identify the current candidate, not a finalized release contract.

Further incompatible design changes can remain part of ABI v3 while it is WIP;
do not infer ABI v4 solely because a provisional v3 layout changes. Keep core
generation, discovery and admission checks coherent so incompatible binaries
are rejected, and rebuild all participants from the same source revision.
Only describe ABI v3 as final or frozen after an explicit release freeze.

Published SDK `0.3.0` remains ABI v2/core generation 19. Keep its released
contract distinct from the evolving v3 candidate.

Review reports remain local and ignored; never commit them or put them in Git history.

# Lifetime ownership contract

Ownership policy is established by construction and the declared ABI input/result
contract. Shared raw weak inputs point to NcSharedLifetimeState; boxed ABI inputs
require boxed backing. Provider metadata has no ownership tag. Do not reintroduce
raw expected-kind checks or mixed-policy rejection/cleanup dispatch: a wrong-policy
input violates its producer contract. Keep shape/interface checks, reference bounds,
terminal expiration and actual reader/writer admission. Core 21 supersedes core 20
inside WIP ABI v3 because type information shrank to four words.
