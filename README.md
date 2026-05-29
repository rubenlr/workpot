

[![CI](https://github.com/rubenlr/workpot/actions/workflows/ci.yml/badge.svg)](https://github.com/rubenlr/workpot/actions/workflows/ci.yml)

## Releasing

See [CONTRIBUTING.md](CONTRIBUTING.md) for PR gates and semver policy. To cut a
release locally, run `bin/release X.Y.Z` then push the commit and tag. GitHub
Actions publishes macOS tarballs to [GitHub Releases](https://github.com/rubenlr/workpot/releases).
Future Tauri tray builds are documented in [docs/releasing.md](docs/releasing.md).