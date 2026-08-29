# Minicraft+ 2.2.4 compatibility report

## Reference and custody

The source baseline is upstream `2.2.X` commit
`47227a89fe05f1e3fe962f070f25c53111254a1f`. The release oracle is the official
9,393,852-byte `minicraft-plus-2.2.4.jar` with SHA-256
`90d534d346eca5da3a200d2c5b6007be919ee94908558c8e75a76c704f9a3f44`.
`scripts/audit_official.py` checks all 403 copied resources against that JAR;
binary content is exact and text content is exact after CRLF normalization.

The executable embeds every required built-in texture, locale, book, content
definition, controller mapping, and sound. It never reads from the sibling Java
source tree. Folder/ZIP resource packs, custom skins, settings, and saves are
optional user data under the selected Rust save directory.

## Behavior checklist

| 2.2.4 area | Rust result |
| --- | --- |
| Window and rendering | Native 288×192 logical framebuffer, copied pixel art/font, alpha and connective-tile composition |
| Front end | Title, world selection/creation, options, help books, achievements, key binding, pause and inventory flows |
| Settings and content | 16 locales with English fallback, four skins, folder/ZIP packs, five tutorials, fourteen quests, seventeen achievements |
| World | Six depths, three sizes, all themes/types, Java-compatible RNG/noise, structures, stairs, day clock and lighting |
| Tiles | IDs 0–58, connector 255, legacy mappings, variants, random ticks, collision, damage, fluids, crops and doors |
| Entities and combat | Stable arenas, natural species, bosses, furniture, occupants, drops, projectiles, particles, armor and potions |
| Items and progression | Full item/tool identities, stations/recipes, food/hunger, fishing, farming, containers and boss progression |
| Modes | Survival, creative, hardcore and timed score behavior |
| Saves | Versioned complete snapshots, deterministic RNG, atomic backup/recovery and first-load Java 2.2.4 import |
| Input and audio | Keyboard on all targets; XInput/WinMM on Windows; SDL controller/audio backends on Linux and macOS; ten embedded WAVs |
| Multiplayer | Port 4225 standalone server and bounded versioned Rust JSON-lines client transport |
| Distribution | Locked Windows, Linux and macOS CI builds, embedded-resource self-check and deterministic self-contained ZIPs |

## Deliberate compatibility boundaries

- The implementation is a clean Rust reconstruction, not a JVM, JNI bridge,
  embedded JAR, or instruction-for-instruction translation. Save output and RNG
  continuation are deterministic within this Rust format, but native save JSON
  is not intended for import back into Java.
- The Java 2.2.4 server is an empty shell and its tree retains only a protocol
  registry. The completed Rust JSON-lines transport therefore does not claim
  wire interoperability with a released Java multiplayer service.
- Atomic `.bak` recovery, strict resource-size/protocol limits, and the
  `--self-check` command are safety/release additions rather than Java behavior.
- Resource-pack and custom-skin overrides remain external by design. Built-in
  assets are embedded, so an empty working directory is a supported launch
  environment.

## License and credits

This reconstruction and the locally copied Minicraft+ resources are distributed
under GPL-3.0; the complete license is included as `LICENSE` in every archive.
Original Minicraft was created by Markus Persson, and Minicraft+ is maintained
by the MinicraftPlus community. The copied upstream `credits.txt` and About book
remain embedded and visible in the game.
