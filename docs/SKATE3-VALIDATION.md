# Validation — 2026-10-28

Environment: Windows x64, Rust stable 1.99.1, MSVC 14.44, native Windows CMake.
The Rust toolchain and Cargo build cache were installed in the task workspace;
the system PATH was not changed.

| Check | Result |
| --- | --- |
| `cargo check --locked -p omsi-skate` | Passed |
| `cargo check --locked --offline -p omsi-app --features skate3 --all-targets` | Passed |
| `cargo check --locked --offline -p omsi-app` | Passed |
| `cargo test --locked --offline -p omsi-skate --lib` | 5 passed |
| `python -m unittest discover -s scripts/tests -p test_prepare_skate_assets.py -v` | 3 passed |
| `scripts/arch-guard.sh` Python body, executed with native Python on Windows | Passed: 45 crates, app 78,348 Rust lines |
| `git -c core.whitespace=-blank-at-eof diff --cached --check` | Passed; existing blank EOF lines in vendored source retained |
| SHA-256 comparison of every vendored Rust crate file against the pinned mashup checkout | Identical |

The Rust tests cover preservation of metres and large world coordinates,
outward obstacle winding, refusal to simulate unloaded tiles, separate terrain
and elevated-road height layers, and rejection of distant collision obstacles.
The Python tests cover refusing an existing output without deleting its files,
refusing incomplete disc data without publishing an output, and refusing to
write converted content inside the original game directory.

The first full typecheck encountered an environment issue: PATH selected the
MSYS CMake, which cannot generate Visual Studio projects for OpenXR. Re-running
with the installed native Windows CMake resolved it. The build retains warnings
from existing openOMSI and unmodified vendored code.

Not performed: release executable linking, original game asset conversion,
GPU/model visual verification, controller playtest, bus → skate → bus gameplay,
moving-vehicle collision, or LAN synchronization. No original game paths were
provided, and OMSI 2 was absent from the discovered Steam library. Passing
typechecks and synthetic tests does not establish that the integration is playable.
