# Phase 8 release parity and hardening report

## Official reference audit

The official 2.2.4 release JAR is identified by a pinned SHA-256 rather than a
mutable filename. The custody script verifies its digest and all 403 resources
copied into this repository. Binary files compare byte-for-byte and text files
compare after CRLF normalization, accounting only for Git's Windows checkout
conversion.

## Robustness and determinism

- Native save mutation tests perform 512 deterministic truncation, bit-flip,
  and invalid-byte cases under panic capture.
- Save loading, Java migration, settings, folder/ZIP resource packs, and network
  frames reject oversized inputs before unbounded allocation.
- Network lines are limited to 64 KiB and still enforce username/content
  validation.
- A save/resume fork advances identically, including complete world state and
  RNG, and a lightweight 129,600-tick two-day soak stays bounded.
- The soak exposed and fixed an eager `then_some` edge-coordinate conversion;
  adjacency generation now evaluates coordinates only after bounds checks and
  water-radius arithmetic is saturating.

## Portable runtime and release process

Windows retains native WinMM audio and dynamically loaded XInput. Linux and
macOS use a bundled SDL2 build for queued audio, game-controller discovery,
hotplug, mappings, axes, and buttons. All ten WAV resources and the controller
mapping database remain compile-time embedded.

`--self-check` loads configuration and embedded assets without a window, checks
every locale and bundled content count, and validates every WAV. CI runs tests,
strict Clippy, a locked release build, and this smoke test on Windows, Ubuntu,
and macOS. Tag builds create deterministic, self-contained ZIP archives; sorted
entries, fixed timestamps/permissions, `Cargo.lock` digest, revision metadata,
license, and compatibility documents make package inputs auditable.

## Acceptance evidence

- Official JAR digest and 403-resource custody audit: clean.
- `cargo fmt --all -- --check`: clean.
- Full Rust test suite: 65 passed, 0 failed in 40.62 seconds.
- `cargo clippy --all-targets --locked -- -D warnings`: clean.
- `cargo build --release --locked`: clean.
- Two successive Windows package runs produced an identical archive SHA-256;
  the package script prints the final digest for release publication.
- The archive was extracted under `target/phase8-unpacked`; its executable
  passed `--self-check` with that extracted directory as the working directory.
- The repository CI/release matrices execute the same locked test/build/smoke/
  package path on Windows, Ubuntu, and macOS; platform artifacts are uploaded
  without depending on the Java repository.
