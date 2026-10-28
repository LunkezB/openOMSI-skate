# openOMSI × Skate 3: experimental in-process integration

This fork connects openOMSI's loaded world to the real Skate 3 Rust rewrite
simulation. The bus, map, traffic and skateboard share one openOMSI process
and window. It does not launch a second game or export maps to another game.

**Status: experimental Windows build, tested locally with original game data
on Grundorf.** The bus-to-skate transition, keyboard pushing and returning to
the bus were exercised in the game. The owned-asset simulation probe also
exercised a Flick-It jump. Exact checks and limits are recorded in
`docs/SKATE3-VALIDATION.md`.

## Build and run on Windows x64

Use current stable Rust (MSVC), Visual Studio C++ Build Tools, Windows SDK,
native Windows CMake and Python 3.13. Avoid an MSYS CMake earlier on PATH: it
cannot generate the Visual Studio project used by the OpenXR dependency.

```powershell
cargo build --locked --release -p omsi-app --features skate3 --bin openomsi
```

For Skate data conversion install the two Python packages used by the pinned
source converter, preferably in a virtual environment:

```powershell
python -m venv .venv-skate
.venv-skate\Scripts\python -m pip install numpy==2.2.6 Pillow==11.3.0
.venv-skate\Scripts\python scripts/prepare-skate-assets.py `
  --xex "D:\Games\Skate3\default.xex" --out ".\skate-data"
```

Supply your extracted Xbox 360 game with its `data` folder beside `default.xex`.
The output must be a new folder outside the source game. Conversion reads the
game and writes to a temporary directory, then publishes the completed folder.
It refuses to replace an existing output. No game data is downloaded.
Already converted assets from the compatible mashup revision can also be used:
point to the `assets` directory containing `private/game.json`.

```powershell
$env:OMSI_SKATE_ASSETS = (Resolve-Path .\skate-data\assets).Path
$env:OMSI_NO_UPDATE = "1"
.\target\release\openomsi.exe --root "D:\Games\OMSI 2" `
  --map "maps/Grundorf/global.cfg" --bus "Vehicles/MAN_SD200/MAN_SD80.bus"
```

The first Skate load can take time. Enable **Ability to get up** in openOMSI's
gameplay settings. With a stationary bus, press **J**: the existing on-foot
system steps outside, then the Skate session loads. If the door cannot be
reached, leave the bus with Ctrl+Shift+G first and press J outdoors.
You can also start with `--on-foot` and press J after terrain loads.

An **Xbox/XInput controller** uses the rewrite's original controls:
A pushes, the left stick steers and the right stick performs Flick-It tricks.
With no XInput device connected, the keyboard supplies the same raw inputs:

| Key | Action |
| --- | --- |
| W / Space | Push |
| A / D | Steer |
| S | Brake |
| Shift | Push with the other foot |
| Arrow keys | Right-stick Flick-It input; down then quickly up performs an ollie |
| Q / E | Left / right grab trigger |
| F | Native board toggle (upstream off-board limitations still apply) |

J dismounts to walking. Walk to your driver's position and press G to return to
the bus. Escape → Back to my bus also returns to the cab. Pause and the Escape
menu keep working. Skate inputs are withheld from
openOMSI's bus controls while the skating session or exit request is active.
`OMSI_NO_UPDATE` keeps upstream release updates from replacing the fork.

## What is connected

* `omsi-skate` runs the pinned `skate-host::bridge::Session` on a worker with a
  32 MiB stack. The simulation advances at the period supplied by that engine.
  A bounded queue prevents input requests accumulating without limit.
* The bridge uses a local origin and a rotation from OMSI's Z-up metres to
  Skate's Y-up metres. Large geographic map coordinates stay in f64 in OMSI.
* Collision includes terrain, drivable road/bridge faces, actual scenery
  collision meshes, static bounding boxes and nearby vehicles. It covers a
  96 m neighbourhood and refreshes after streaming changes or movement.
* Ledge grind rails are detected by the existing mashup algorithm on the worker.
* The converted skater-and-board GLB is CPU-skinned with the rewrite's solved
  bone matrices and drawn by openOMSI's renderer. The rewrite supplies the camera.
* J drops the Skate worker and returns to the existing walker. Boarding and
  driving then use openOMSI's existing doors, seats and vehicle logic.

## Current limits

* Windows x64, XInput and keyboard fallback are supported. Other controllers
  and mobile controls have not been connected or tested.
* Vehicle collision is refreshed as static geometry at intervals: moving-platform
  velocity transfer and two-way collision impulses are not implemented.
* Collision comes from OMSI simulation surfaces and collision meshes, which may
  be simpler than the visible scenery. No guarantee is made for arbitrary maps.
* The GLB's base colour/textures and skin are connected. Morph targets, the full
  Skate material shaders, skating audio, scoring HUD and replays are not connected.
* LAN peers see the existing walker state, not a synchronized Skate skeleton.
  Skate multiplayer is not implemented; use local single-player for playtests.
* The source uses the compatible mashup extraction at the revisions listed in
  `vendor/skate/NOTICE.md`; it does not claim parity with the latest SK8-ENGINE.
* Initial model decoding, shader compilation, collision gathering and CPU
  skinning may cause stalls. The first renderer startup took about 40–50 seconds
  on the tested integrated GPU; a loading window is normal during that time.
* Only Grundorf was played locally. Grinds on arbitrary scenery, long streaming
  sessions, the physical controller path and more demanding maps remain unverified.

## Owned-asset diagnostics

```powershell
cargo run --locked -p omsi-skate --example check_session -- "path/to/assets"
```

This loads the real graphs, animation banks and physics, pushes on a flat
collision world for 360 ticks, then performs a Flick-It jump. It fails if the
worker errors, pushing does not move the skater, or the jump does not lift it.

## Related work checked on 2026-10-28

Targeted searches for `openOMSI Skate 3`, `openOMSI skate3` and
`openOMSI skateboard` did not reveal an existing public integration. This is a
search result, not proof that no private or unindexed project exists.

The closest public implementations are:

* [2010 Rust Rewrite Mashup](https://github.com/chasmlol/2010-rust-rewrite-mashup),
  particularly [Skate inside MW2](https://github.com/chasmlol/2010-rust-rewrite-mashup/blob/main/docs/SKATE.md).
  This supplies the reusable in-process host used here.
* [SkateGM](https://github.com/the-schwilliam/SkateGM): Skate 3 mechanics inside
  Garry's Mod, including a moving-collision extension worth evaluating later.
* [Skate 3 Rust Engine](https://github.com/SK8-ENGINE/skate-3-rust-engine):
  the original Rust/Bevy skating project, built on dumbad's research.

The base openOMSI commit is `0301e4b427bfba7d71a491c88df1cad5ca254ff2`.
