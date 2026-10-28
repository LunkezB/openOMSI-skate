# Validation — 2026-10-28

Environment: Windows x64, Rust stable 1.99.1, MSVC 14.44, native Windows CMake.
The Rust toolchain and Cargo build cache were installed in the task workspace;
the system PATH was not changed.

| Check | Result |
| --- | --- |
| `cargo check --locked -p omsi-skate` | Passed |
| `cargo check --locked --offline -p omsi-app --features skate3 --all-targets` | Passed |
| `cargo check --locked --offline -p omsi-app` | Passed |
| `cargo build --locked --offline -p omsi-app --features skate3 --bin openomsi` | Passed; Windows x64 dev executable linked |
| `cargo test --locked --offline -p omsi-skate --lib` | 5 passed |
| `cargo run --locked --offline -p omsi-skate --example check_session -- <owned assets>` | Passed: 480 ticks, 31.662 m push distance, 1.170 m Flick-It rise |
| `python -m unittest discover -s scripts/tests -p test_prepare_skate_assets.py -v` | 3 passed |
| `scripts/arch-guard.sh` Python body, executed with native Python on Windows | Passed: 45 crates, app below the 81,600-line limit |
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

## Original-data playtest

After the initial source implementation, the user supplied local OMSI 2 and
Skate 3 paths. The Skate folder held an Xbox 360 ISO. It was extracted with
XboxDev/extract-xiso, verified against the converter's pinned SHA-256, and the
asset preparation script completed with the pinned NumPy/Pillow dependencies.
Original game data and screenshots remain local and are excluded from Git.

On Grundorf, the Windows executable loaded the MAN SD80 driver's view. The
standard Gameplay menu enabled getting up. J walked out of the bus and loaded
a Skate session, including a textured skater-and-board GLB and the native camera.
No XInput device was present, so keyboard fallback was exercised: holding W for
three seconds moved the skater from (505.4723, 739.1869, 9.0870) to
(527.7714, 734.8672, 9.0870), a displacement of 22.7136 m measured across the
five-second observation interval. J returned to walking. The existing
"Back to my bus" menu action restored the driver's view and logged
"Back at the wheel". The session exited normally, without Skate errors or panics.

The separate owned-asset worker probe ran 480 simulation ticks, checked finite
pose outputs and ground support, pushed 31.662 m, and performed a Flick-It jump
with a 1.170 m rise before returning to PhysicsGround.

The first timed window test ended after 50 seconds while shaders were still
compiling; subsequent tests used a longer timeout. A test-script key name was
also corrected from `KeyJ` to the script parser's `J` spelling. These were test
setup issues. Keyboard fallback and cleanup of skater graphics on alternate
bus/vehicle transitions were added during this pass.

Not performed: optimized release linking, physical controller playtest,
boarding specifically through the G-at-door interaction, grinds on real OMSI
scenery, extended tile streaming, moving-platform impulse transfer, or LAN
synchronization. The implementation remains experimental and was played only
on Grundorf. The final feature/default typechecks and five Rust tests passed
again after the keyboard and graphics-cleanup changes.
