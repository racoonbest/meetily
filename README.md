# Memo

Local meeting transcripts, notes, and summaries.

Memo is an independent fork of [Meetily Community Edition](https://github.com/Zackriya-Solutions/meetily), starting from its `main` branch with the existing always-dark interface. It retains the Rust/Tauri desktop app, Next.js interface, local Whisper and Parakeet transcription, and summary integrations.

## Current foundation

- App and window name: **Memo**
- Application identifier: `com.racoonbest.memo`
- Always-dark interface
- Audio-file saving off by default; transcription remains available
- Manual application updates

Memo uses its own application data directory. It does not automatically move or overwrite an existing Meetily library. On macOS, Memo's data is stored in `~/Library/Application Support/com.racoonbest.memo`; optional audio recordings default to `~/Movies/memo-recordings`.

## Development

```sh
git clone https://github.com/racoonbest/memo.git
cd memo/frontend
pnpm install --frozen-lockfile
pnpm tauri:dev
```

Native builds require Rust and the platform build tools described in [Building from Source](docs/BUILDING.md). Run `pnpm build` for the frontend build and `pnpm test:theme` for the existing dark-theme checks.

For upstream updates, add Meetily as a separate remote and review its changes before merging:

```sh
git remote add upstream https://github.com/Zackriya-Solutions/meetily.git
git fetch upstream
```

## Attribution and license

The original [MIT license](LICENSE.md) and copyright notice are preserved. Dependencies and downloaded models retain their respective licenses. Inherited artwork and historical documentation remain from Meetily; they can be revised as Memo develops. Meetily Pro is a separate product and is not included in this repository.
