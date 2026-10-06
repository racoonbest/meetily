# Always-dark build

This fork starts from upstream `main` and keeps the interface dark, including
the native window, menus, editors, dialogs, and notifications. It is an unofficial
Community Edition build. Existing license and copyright notices still apply.

The property-specific Tailwind palette in `frontend/theme/always-dark.cjs` maps
light surfaces and dark text to dark surfaces and light text. It preserves white
button labels and colored action backgrounds. Semantic CSS variables cover the
shared controls. There is no appearance selector.

## Build and verify

Use the existing **Build and Test - macOS** GitHub Actions workflow. Select a
release build with signing disabled and artifact upload enabled. Apple developer
and updater signing secrets are not required. The artifact includes
`meetily-dark.zip`, preserving executable permissions.

Locally, install the committed dependencies with pnpm 9.15.9 and run
`pnpm test:theme`, `pnpm exec tsc --noEmit`, and `pnpm build` from `frontend`.
The upstream source-build instructions still apply for a native build.

Production builds use manual updates so the upstream updater cannot replace the
custom theme. Merge desired upstream changes into the fork and build again.

Before replacing an installed app, quit it and back up its application bundle
and app-data directory. The bundle identifier is unchanged so existing local
meetings and settings remain available. No user data belongs in this repository.
