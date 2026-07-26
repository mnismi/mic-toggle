# Contributing to mic-toggle

Thanks for your interest! Bug reports, feature suggestions, and pull
requests are all welcome.

## Prerequisites

- Windows 10/11
- Stable [Rust](https://rustup.rs/) with the MSVC toolchain

## Building and running

    cargo run            # debug build, starts the tray app
    cargo build --release

## Tests and checks

CI runs these on every push and pull request; please make sure they pass
locally first:

    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    cargo test

Two caveats about the test suite:

- **Quit mic-toggle before running tests.** The hotkey test registers F8
  and fails with "Hot key is already registered" while the app is running.
- The audio test is `#[ignore]`d because it needs a real capture device
  (CI runners have none) and briefly mutes your actual microphone. Run the
  full suite locally with:

      cargo test -- --include-ignored

## Pull requests

1. Fork and create a feature branch off `main`.
2. Keep changes focused — one topic per PR.
3. Follow the existing commit style: `feat:`, `fix:`, `docs:`, `chore:`
   prefixes with a short imperative summary.
4. Make sure fmt, clippy, and tests pass (see above).

Every push to `main` automatically rebuilds the exe and republishes the
rolling [`latest` release](https://github.com/mnismi/mic-toggle/releases/tag/latest),
so merged changes ship immediately.

## License

By contributing, you agree that your contributions will be licensed under
the [MIT License](LICENSE).
