# 2.2.4 baseline audit

| Property | Verified value |
| --- | --- |
| Source branch | `MinicraftPlus/minicraft-plus-revived:2.2.X` |
| Source commit | `47227a89fe05f1e3fe962f070f25c53111254a1f` |
| Source version declarations | `2.2.4` in `build.gradle` and `Game.java` |
| Client Java files | 210 |
| Client resource files | 403 |
| Copied resource bytes | 1,764,651 |
| Logical framebuffer | 288×192 |
| Simulation baseline | 60 ticks/second |
| Levels | depths `-4`, `-3`, `-2`, `-1`, `0`, `1` |
| Tile registry | IDs 0–58 plus internal connector 255 |

The official `v2.2.4` release was published with a resource-pack startup fix.
Its tag points to a different development line, so it is not used as a source
tree. The release artifact remains the final black-box behavior oracle.

