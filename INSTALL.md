# Install Workpot (macOS)

Workpot supports two first-class install paths:

1. **One-line installer script** (installs latest release assets)
2. **Manual DMG install** (drag-install app bundle)

Both paths use the same GitHub Release artifacts.

## Option A: One-line installer script

### Convenience URL (latest installer script on `main`)

```bash
curl -fsSL https://raw.githubusercontent.com/rubenlr/workpot/main/scripts/install.sh | bash
```

### Versioned release URL (reproducible installer per release tag)

```bash
curl -fsSL https://github.com/rubenlr/workpot/releases/download/vX.Y.Z/install.sh | bash
```

Replace `vX.Y.Z` with the release tag you want.

### Installer flags

- Default (no flags): installs both CLI + tray app
- `--only-cli`: installs only the CLI binary
- `--only-tray`: installs only the tray app
- `--global`: installs to system locations (`/usr/local/bin`, `/Applications`)

Examples:

```bash
# CLI only (user install)
curl -fsSL https://raw.githubusercontent.com/rubenlr/workpot/main/scripts/install.sh | bash -s -- --only-cli

# Tray only (global install)
curl -fsSL https://raw.githubusercontent.com/rubenlr/workpot/main/scripts/install.sh | bash -s -- --only-tray --global
```

## Option B: Manual DMG install

1. Open [GitHub Releases](https://github.com/rubenlr/workpot/releases).
2. Download `Workpot-X.Y.Z-aarch64.dmg` and (recommended) `Workpot-X.Y.Z-aarch64.dmg.sha256`.
3. Optional integrity check:

   ```bash
   shasum -a 256 Workpot-X.Y.Z-aarch64.dmg
   # compare with Workpot-X.Y.Z-aarch64.dmg.sha256
   ```

4. Open the DMG.
5. Drag `Workpot.app` into `Applications`.
6. Launch Workpot from Spotlight/Finder.

## Install locations

Default user install:

- CLI: `~/.local/bin/workpot`
- Tray: `~/Applications/Workpot.app`

Global install (`--global`):

- CLI: `/usr/local/bin/workpot`
- Tray: `/Applications/Workpot.app`

## Update

Update to latest release:

```bash
workpot update
```

Useful flags:

- `workpot update --only-cli`
- `workpot update --only-tray`
- `workpot update --global`

If Workpot tray is currently running, `workpot update` may require you to quit the app first.

## Uninstall

Remove CLI:

```bash
rm -f ~/.local/bin/workpot
# or global
sudo rm -f /usr/local/bin/workpot
```

Remove tray app:

```bash
rm -rf ~/Applications/Workpot.app
# or global
sudo rm -rf /Applications/Workpot.app
```

Optional: remove local config/data created by Workpot:

```bash
rm -rf ~/Library/Application\ Support/workpot
rm -rf ~/.config/workpot
```

## PATH troubleshooting

If `workpot` is not found after install:

1. Confirm binary exists:

   ```bash
   ls -l ~/.local/bin/workpot
   ```

2. Add `~/.local/bin` to your shell PATH (zsh):

   ```bash
   echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
   source ~/.zshrc
   ```

3. Verify:

   ```bash
   workpot --version
   ```
