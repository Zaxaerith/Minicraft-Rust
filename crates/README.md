# Minicraft Rust workspace

This directory is the staged destination for the Java-parity architecture.

- `minicraft-common`: versioned data and protocol types shared by client and server.
  - `protocol.rs`: Java `MinicraftProtocol` registry order, port 4225, login/heartbeat
    message types, and bounded line-delimited JSON wire helpers.
- `minicraft-client`: the client module tree corresponding to the Java client packages.
  - `network/transport.rs`: `MultiplayerClient` TCP transport only (no server assets).
  - `level/`: WorldSpec/GameMode, Tile registry, JavaRandom, LevelGen, map stamping.
  - `item/`, `resource/`, `progression/`, `io/settings`, `io/audio/sound`, `saveload/storage`.
- `minicraft-server`: standalone multiplayer server (session + TCP listener + binary).
  - Depends only on `minicraft-common`; ships no worlds, textures, or client assets.
- `minicraft-parity`: headless adapters and JSONL differential-test runner.

During the migration, the root `minicraft-rust` package remains the runnable
compatibility client and the default workspace member. Empty module roots
declare ownership boundaries, not completed implementations.

Stage 14 (2026-09-09): `src/network.rs` was removed. CLI `--server` and
`--multiplayer-probe` re-export from the server/client crates. Prefer
`cargo run -p minicraft-server` for a console-attached listen process on
Windows (the windowed client subsystem hides stdout).

As of 2026-09-09, shared Clock/Updater ownership, Display transitions, input
state and PCM mixing have moved into the client tree. Stage 5 is moving graphics:
Image, Screen, the font character table and shared GUI frame/title painting now
live in `minicraft-client/src/gfx`. Concrete layouts still use root adapters.
Stages 3 and 4 retain deferred lifecycle/input/audio gaps; full Java parity is
not yet established. See the root [README](../README.md) for current scope.

The parity runner source is retained because it is a Cargo workspace member.
Its local Java oracle tools, trace fixtures and generated reports are deliberately
not published. Building the game does not require those local directories.
