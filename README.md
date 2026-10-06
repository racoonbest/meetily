# Memo

Local meeting transcripts, notes, and summaries.

Memo is an independent fork of [Meetily Community Edition](https://github.com/Zackriya-Solutions/meetily), starting from its `main` branch with the existing always-dark interface. It retains the Rust/Tauri desktop app, Next.js interface, local Whisper and Parakeet transcription, and summary integrations.

## Install on macOS

**Requires an Apple Silicon Mac (M1 or later), macOS 14.4 or later, and an internet connection.** Intel Macs, Windows, and Linux are not supported by this installer.

Open **Terminal**, paste this one line, and press **Return**:

```sh
curl -fsSL https://raw.githubusercontent.com/racoonbest/memo/main/install.sh | /bin/bash
```

The command downloads the latest published Memo app, checks its SHA-256 checksum and code signature, installs it in **`~/Applications/Memo.app`**, and opens it. No Homebrew, developer tools, GitHub account, or administrator password is needed. You can inspect [the installer](install.sh) before running it.

### First launch

1. Allow **Microphone** access when Memo requests it. Allow **System Audio Recording** or **Screen & System Audio Recording** if you want to transcribe computer audio; macOS wording varies by version.
2. In **Settings → Transcription**, download a local transcription model and wait until it is ready. Models require an additional download and disk space.
3. Select your audio inputs and start a meeting. Audio-file saving is **off by default**; transcripts are still saved.

This community build is **ad-hoc signed, not Apple-notarized**. If macOS blocks the first launch, open **System Settings → Privacy & Security → Open Anyway** for Memo, then confirm **Open**. Only approve a download you trust. See [Apple's instructions](https://support.apple.com/en-us/102445). The installer does not disable Gatekeeper or change macOS security settings.

### Updates and existing data

Quit Memo and run the same command again to install the latest release. The previous app is kept in `~/Applications/.memo-backups/`; the installer prints its exact location. To roll back, quit Memo and move the preserved `Memo.app` back into `~/Applications`.

Installation and updates leave your meetings, models, and settings alone. Memo has a separate data directory from Meetily; installing Memo does not import or overwrite your Meetily library.

### Other installation options

Download the script once if you want to review it or change the destination:

```sh
curl -fsSL https://raw.githubusercontent.com/racoonbest/memo/main/install.sh -o install.sh
bash install.sh --dir /Applications --no-open
```

Use a directory you can write to; do not run the installer with `sudo`. `--no-open` installs without launching the app. `bash install.sh --check` downloads and verifies the release without installing it.

For a manual install, download **`Memo-macos-arm64.zip`** from [Releases](https://github.com/racoonbest/memo/releases/latest), unzip it, and move **Memo.app** to your Applications folder. Quit an existing Memo before replacing it.

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
