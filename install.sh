#!/bin/bash
# Download a prebuilt macOS release using only tools included with macOS.

main() {
  set -euo pipefail

  # Memo release profile. Keep distribution-specific defaults together.
  local repository="racoonbest/memo"
  local app_name="Memo"
  local bundle_id="com.racoonbest.memo"
  local executable="memo"
  local asset="Memo-macos-arm64.zip"
  local minimum_major=14 minimum_minor=4
  local install_dir="$HOME/Applications" launch=1 check_only=0
  local work_dir="" stage_dir="" lock_dir="" backup="" destination=""

  fail() { printf 'Memo installer: %s\n' "$*" >&2; exit 1; }
  cleanup() {
    local result=$?
    trap - EXIT
    if [[ -n "$backup" && -d "$backup" && ! -e "$destination" ]]; then
      mv "$backup" "$destination" || printf 'Restore your previous app from: %s\n' "$backup" >&2
    fi
    [[ -z "$stage_dir" ]] || rm -rf "$stage_dir"
    [[ -z "$work_dir" ]] || rm -rf "$work_dir"
    [[ -z "$lock_dir" ]] || rmdir "$lock_dir"
    exit "$result"
  }

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --dir)
        [[ $# -ge 2 && -n "$2" ]] || fail "--dir requires an absolute directory."
        install_dir="$2"; shift 2 ;;
      --no-open) launch=0; shift ;;
      --check) check_only=1; shift ;;
      --help)
        printf '%s\n' "Usage: bash install.sh [--dir /absolute/path] [--no-open] [--check]" \
          "Default: install to ~/Applications and open Memo." \
          "--check downloads and verifies the release without installing or opening it."
        return 0 ;;
      *) fail "Unknown option: $1. Run with --help for usage." ;;
    esac
  done

  [[ "$(uname -s)" == Darwin ]] || fail "This installer supports macOS only."
  [[ "$(uname -m)" == arm64 || "$(sysctl -in hw.optional.arm64 2>/dev/null || true)" == 1 ]] ||
    fail "This release requires Apple Silicon (M1 or later). Intel builds are not available."
  local os_version major minor
  os_version=$(sw_vers -productVersion)
  IFS=. read -r major minor _ <<< "$os_version"
  [[ "$major" =~ ^[0-9]+$ && "$minor" =~ ^[0-9]+$ ]] || fail "Cannot read the macOS version."
  (( major > minimum_major || (major == minimum_major && minor >= minimum_minor) )) ||
    fail "macOS ${minimum_major}.${minimum_minor} or later is required; found $os_version."
  [[ "$install_dir" == /* ]] || fail "--dir must be an absolute path."
  [[ $EUID -ne 0 ]] || fail "Run this command as your normal user, without sudo."

  work_dir=$(mktemp -d "${TMPDIR:-/tmp}/memo-install.XXXXXX")
  trap cleanup EXIT
  trap 'exit 130' INT
  trap 'exit 143' TERM

  download() {
    curl --fail --location --show-error --silent --retry 3 --connect-timeout 20 \
      --max-time 900 --proto '=https' --proto-redir '=https' "$1" -o "$2"
  }

  printf 'Finding the latest Memo release…\n'
  download "https://api.github.com/repos/$repository/releases/latest" "$work_dir/release.json" ||
    fail "Cannot find a published release. Check https://github.com/$repository/releases and try again."
  local tag base_url
  tag=$(plutil -extract tag_name raw -o - "$work_dir/release.json") || fail "Invalid release metadata."
  [[ "$tag" =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ ]] || fail "Invalid release tag."
  # Resolve the tag once so a newly published release cannot mix archive/checksum versions.
  base_url="https://github.com/$repository/releases/download/$tag"
  printf 'Downloading Memo %s…\n' "$tag"
  download "$base_url/$asset" "$work_dir/$asset" || fail "App download failed; nothing was installed."
  download "$base_url/$asset.sha256" "$work_dir/$asset.sha256" || fail "Checksum download failed; nothing was installed."

  local expected actual checksum_name extra
  read -r expected checksum_name extra < "$work_dir/$asset.sha256" || fail "Empty checksum file."
  [[ "$expected" =~ ^[A-Fa-f0-9]{64}$ && "$checksum_name" == "$asset" && -z "$extra" ]] ||
    fail "Invalid checksum file."
  actual=$(shasum -a 256 "$work_dir/$asset")
  [[ "${actual%% *}" == "$expected" ]] || fail "Checksum mismatch; nothing was installed."

  mkdir "$work_dir/unpacked"
  ditto -x -k "$work_dir/$asset" "$work_dir/unpacked"
  local source="$work_dir/unpacked/$app_name.app"
  [[ -d "$source" && ! -L "$source" && -x "$source/Contents/MacOS/$executable" ]] || fail "Invalid app archive."
  [[ "$(plutil -extract CFBundleIdentifier raw -o - "$source/Contents/Info.plist")" == "$bundle_id" ]] ||
    fail "The downloaded app has an unexpected identity."
  codesign --verify --deep --strict "$source" || fail "The app's code signature is invalid."
  local version
  version=$(plutil -extract CFBundleShortVersionString raw -o - "$source/Contents/Info.plist")
  printf 'Verified Memo %s (%s): SHA-256 and app signature passed.\n' "$version" "$tag"
  if [[ $check_only == 1 ]]; then
    exit 0
  fi

  pgrep -x "$executable" >/dev/null && fail "Quit Memo before updating, then run this command again."
  mkdir -p "$install_dir"
  [[ -w "$install_dir" ]] || fail "$install_dir is not writable. Use --dir with a directory you own."
  if mkdir "$install_dir/.memo-install.lock" 2>/dev/null; then
    lock_dir="$install_dir/.memo-install.lock"
  else
    fail "Another installation is running (lock: $install_dir/.memo-install.lock)."
  fi
  destination="$install_dir/$app_name.app"
  [[ ! -L "$destination" ]] || fail "Refusing to replace an app symlink: $destination"
  if [[ -e "$destination" ]]; then
    [[ -d "$destination" && "$(plutil -extract CFBundleIdentifier raw -o - "$destination/Contents/Info.plist")" == "$bundle_id" ]] ||
      fail "Refusing to replace an unrelated file or app: $destination"
  fi
  stage_dir=$(mktemp -d "$install_dir/.memo-stage.XXXXXX")
  ditto "$source" "$stage_dir/$app_name.app"
  codesign --verify --deep --strict "$stage_dir/$app_name.app" || fail "The staged app failed verification."
  if [[ -d "$destination" ]]; then
    local backup_dir="$install_dir/.memo-backups/$(date -u +%Y%m%dT%H%M%SZ)-$$"
    mkdir -p "$backup_dir"
    backup="$backup_dir/$app_name.app"
    mv "$destination" "$backup"
  fi
  # Stage on the destination volume, then rename; a failed copy never replaces the old app.
  mv "$stage_dir/$app_name.app" "$destination"
  printf 'Installed Memo %s at %s\n' "$version" "$destination"
  [[ -z "$backup" ]] || printf 'Previous app preserved at: %s\n' "$backup"
  printf 'Your meetings, settings, models, and Meetily installation were left untouched.\n'
  printf 'This community build is ad-hoc signed, not Apple-notarized.\n'
  if [[ $launch == 1 ]]; then
    if ! open "$destination"; then
      printf 'Installed successfully. Open %s from Finder.\n' "$destination"
      printf 'If macOS blocks it, use System Settings → Privacy & Security → Open Anyway.\n'
    fi
  fi
  exit 0
}

main "$@"
