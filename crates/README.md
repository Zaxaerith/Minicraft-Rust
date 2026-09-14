# Minicraft Rust workspace

Java-parity architecture destination for the Minicraft+ 2.2.4 Rust rewrite.

- `minicraft-common`: versioned data and protocol types shared by client and server.
  - `protocol.rs`: Java `MinicraftProtocol` registry order, port 4225, login/heartbeat
    message types, and bounded line-delimited JSON wire helpers.
- `minicraft-client`: the client module tree corresponding to the Java client packages.
  - `app/`: CLI dispatch, display stack host, menu build/render, world record load.
  - `world/`: player tick, level/entities, render, save, HUD, pause, quests.
  - `level/`: WorldSpec/GameMode, Tile registry, JavaRandom, LevelGen, map stamping.
  - `item/`, `resource/`, `progression/`, `io/settings`, `io/audio/sound`, `saveload/storage`.
  - `network/transport.rs`: `MultiplayerClient` TCP transport only (no server assets).
- `minicraft-server`: standalone multiplayer server (session + TCP listener + binary).
  - Depends only on `minicraft-common`; ships no worlds, textures, or client assets.
- `minicraft-parity`: headless adapters and JSONL differential-test runner.

The root `minicraft-rust` package is a thin binary entry (`src/main.rs` +
`src/lib.rs`) that calls `minicraft_client::app::run_cli`. Prefer
`cargo run -p minicraft-server` for a console-attached listen process on
Windows (the windowed client subsystem hides stdout).

The parity runner source is retained because it is a Cargo workspace member.
Its local Java oracle tools, trace fixtures and generated reports are deliberately
not published. Building the game does not require those local directories.
