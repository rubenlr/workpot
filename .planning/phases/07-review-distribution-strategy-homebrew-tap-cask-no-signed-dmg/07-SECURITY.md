---
phase: 07
slug: review-distribution-strategy-homebrew-tap-cask-no-signed-dmg
status: verified
threats_open: 0
asvs_level: 1
created: 2026-06-04
---

# Phase 07 — Security

> Distribution strategy: Homebrew tap + cask, unsigned tarball, CI tap-update. Threat register from plans 07-01–07-04; verified 2026-06-04.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| source code → CLI binary | Removing `reqwest` and update subcommand shrinks network-capable surface in `workpot-cli` | Build artifacts only |
| CI runner → `rubenlr/homebrew-workpot` | `HOMEBREW_TAP_TOKEN` authenticates git push to tap repo | Cask definition (`version`, `sha256`, `url`) |
| GitHub Release → Homebrew install | Published tarball + `.sha256` checksum file | Release binaries (public) |
| `HOMEBREW_TAP_TOKEN` → tap repo | Token with Contents write; leak enables malicious cask | Repository contents |
| Homebrew cask `sha256` → artifact | Integrity gate before install | Checksum vs downloaded tarball |
| docs → user behavior | INSTALL.md migration `rm` instructions | User filesystem paths |

---

## Threat Register

| Threat ID | Category | Component | Disposition | Mitigation | Status |
|-----------|----------|-----------|-------------|------------|--------|
| T-07-01-01 | Tampering | `workpot-cli` dependency removal | mitigate | No `reqwest`/`sha2`/`serde_json` in `crates/workpot-cli`; `update.rs` removed | closed |
| T-07-01-02 | Information Disclosure | HTTP client removed from CLI | accept | Documented — removal reduces attack surface | closed |
| T-07-01-SC | Tampering | cargo installs | accept | Plan scope: deletions only | closed |
| T-07-02-01 | Tampering | `tap-update` patches `Casks/workpot.rb` | mitigate | `grep -q` after `sed` in `.github/workflows/release.yml:235-236` | closed |
| T-07-02-02 | Elevation of Privilege | `HOMEBREW_TAP_TOKEN` | mitigate | D-03: fine-grained PAT scoped to `homebrew-workpot`; workflow uses `secrets.HOMEBREW_TAP_TOKEN` + `repository: rubenlr/homebrew-workpot` (`release.yml:223-226`) | closed |
| T-07-02-03 | Tampering | Artifact substitution vs SHA256 | mitigate | `tap-update` downloads checksum from published release via `gh release download` (`release.yml:218-219`) | closed |
| T-07-02-04 | Tampering | Unsigned artifacts (no DMG) | accept | D-08: intentional unsigned model; no `dmg` in `.github/workflows/` | closed |
| T-07-02-SC | Tampering | marketplace Actions | accept | YAML structure edits only | closed |
| T-07-03-01 | Tampering | INSTALL.md migration `rm` | mitigate | Scoped paths + “run only paths that apply” (`INSTALL.md:46-52`); no wildcards | closed |
| T-07-03-02 | Information Disclosure | Public unsigned-distribution record | accept | Intentional transparency in `docs/distribution-strategy.md` § Security | closed |
| T-07-03-SC | Tampering | package installs | accept | Doc deletions/writes only | closed |
| T-07-04-01 | Tampering | `HOMEBREW_TAP_TOKEN` compromise | mitigate | Same as T-07-02-02 (D-03 operational control) | closed |
| T-07-04-02 | Tampering | Placeholder `sha256` before first release | mitigate | Invalid placeholder hex in `docs/homebrew-tap-files/Casks/workpot.rb:3` | closed |
| T-07-04-03 | Spoofing | `binary` stanza path | mitigate | `#{appdir}/.../workpot` present; no `staged_path` in cask | closed |
| T-07-04-04 | Information Disclosure | Gatekeeper quarantine UX | mitigate | `postflight` xattr in cask (`workpot.rb:18-21`); INSTALL.md fallback (`INSTALL.md:78-81`) | closed |
| T-07-04-SC | Tampering | package installs | accept | Cask/docs only | closed |

*Disposition: mitigate · accept · transfer*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-07-01 | T-07-01-02 | Removing `reqwest` eliminates CLI HTTP update channel; accepted reduction of capability | Phase 07 threat model | 2026-06-04 |
| AR-07-02 | T-07-01-SC | No new dependencies in plan 07-01 | Phase 07 threat model | 2026-06-04 |
| AR-07-03 | T-07-02-04 | D-08: no Apple signing; Homebrew + postflight xattr is the chosen Gatekeeper model | Phase 07 / D-08 | 2026-06-04 |
| AR-07-04 | T-07-02-SC | No new third-party Actions or packages in plan 07-02 | Phase 07 threat model | 2026-06-04 |
| AR-07-05 | T-07-03-02 | Public documentation of unsigned distribution is intentional, not a secret leak | Phase 07 / D-15 | 2026-06-04 |
| AR-07-06 | T-07-03-SC | Documentation-only plan | Phase 07 threat model | 2026-06-04 |
| AR-07-07 | T-07-04-SC | No package installs in plan 07-04 | Phase 07 threat model | 2026-06-04 |

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-06-04 | 16 | 16 | 0 | gsd-security-auditor (orchestrator inline) |

### Security Audit 2026-06-04

| Metric | Count |
|--------|-------|
| Threats found | 16 |
| Closed | 16 |
| Open | 0 |

**Unregistered flags:** None. Summaries 07-01, 07-02 report no new threat flags; 07-03/07-04 document threat coverage inline.

**Residual operational note (not a blocker):** `HOMEBREW_TAP_TOKEN` scope and rotation are enforced outside the repo (GitHub secret configuration per D-03). Re-verify after PAT rotation.

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-06-04
