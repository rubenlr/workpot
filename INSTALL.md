# Install Workpot (macOS)

Workpot is a macOS menu-bar assistant for engineers who juggle many git repositories. A single `brew install` puts both the `workpot` CLI on your PATH and the `Workpot.app` menu-bar tray in `/Applications`.

## Install

```bash
brew tap rubenlr/workpot
brew install rubenlr/workpot/workpot
```

Homebrew automatically removes the quarantine attribute on first install — you will not see an "unidentified developer" dialog.

## Install locations

- CLI: `$(brew --prefix)/bin/workpot` → symlink to `Workpot.app/Contents/MacOS/workpot`
- Tray: `/Applications/Workpot.app`

## Upgrade

```bash
brew upgrade rubenlr/workpot/workpot
```

## Uninstall

```bash
brew uninstall rubenlr/workpot/workpot
```

Optionally remove the tap:

```bash
brew untap rubenlr/workpot
```

Optional data cleanup (removes config and index):

```bash
rm -rf ~/Library/Application\ Support/workpot
rm -rf ~/.config/workpot
```

## Migration from 06.1 install script

If you previously installed Workpot via the install script, remove the old install before switching to Homebrew. Run only the paths that apply to your system:

```bash
rm -f ~/.local/bin/workpot
rm -rf ~/Applications/Workpot.app
rm -f /usr/local/bin/workpot
sudo rm -rf /Applications/Workpot.app
```

Then install via Homebrew:

```bash
brew tap rubenlr/workpot
brew install rubenlr/workpot/workpot
```

## Troubleshooting

**`workpot` not found after install:**

Run `brew doctor` and verify `$(brew --prefix)/bin` is on your PATH:

```bash
echo $PATH | tr ':' '\n' | grep "$(brew --prefix)/bin"
```

If missing, add it to your shell profile (e.g. `~/.zshrc`):

```bash
export PATH="$(brew --prefix)/bin:$PATH"
```

**Workpot shows "damaged" on first launch** (edge case where Homebrew postflight did not fire):

```bash
xattr -dr com.apple.quarantine /Applications/Workpot.app
```
