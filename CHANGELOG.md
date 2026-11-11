# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

_Nothing yet._

## [1.2.2] - 2026-11-11

### Changed

- Label Dependabot pull requests `no-issue-needed`, alongside the default `dependencies` and ecosystem labels, so bot PRs are marked as intentionally having no linked issue.
- README: `lfr-local` is now in the Scoop bucket, and the tap and bucket pick up releases automatically.

### Security

- Upgrade `zip` to 9.x, resolving two Dependabot alerts.
- Skip tar.gz and 7z entries whose paths would escape the extraction directory (`..` components, absolute paths, symlinks or hard links pointing outside it), with or without top-level stripping.

## [1.2.1] - 2026-11-11

### Added

- Add `CHANGELOG.md`, backfilled for earlier releases.
- Add `scripts/release.py` (changelog extraction and annotated tagging) and `scripts/doctor.py` (placeholder check) from the Liferay Rust template v0.7.0.
- Add the shared `.githooks/pre-commit` hook, `.cargo/config.toml` aliases (`setup-hooks`, `lint`, `fmt-check`) and `.github/dependabot.yml`.
- Add `formula.rb.example` for Homebrew distribution.
- Document the release process in a Releasing section of `README.md`.

### Changed

- Align the package version in `Cargo.toml` with the release tags (it still said `1.0.0` after the 1.1.0 and 1.2.0 releases).
- Replace `.github/workflows/release.yml` with the template workflow, which derives names from `Cargo.toml`, rejects tags that are not annotated or don't match the `Cargo.toml` version, and uses the `CHANGELOG.md` section as release notes.
- Pin GitHub Actions in `.github/workflows/rust.yml` to commit SHAs.
- Update `scoop.json.example` and `.gemini/prompts/update-distribution-channels.md` to the template versions.
- Restructure `README.md` to the template standard and correct inaccurate instructions (pre-commit hook setup, Scoop bucket name, archive formats accepted by `init`).
- Bump `openssl` to 0.10.82, `openssl-sys` to 0.9.118, `rand` to 0.8.8 and `tar` to 0.4.46 to address security advisories.

### Fixed

- `lfr-local --version` now reports the package version instead of a hardcoded `0.1.0`.

## [1.2.0] - 2026-03-23

### Added

- Reuse bundles from the Liferay Blade cache (`~/.liferay/bundles`) in `init` before downloading.
- Extract `.7z` and `.tar.gz` bundles as well as `.zip`, stripping the top-level directory for all formats.

## [1.1.0] - 2026-03-23

### Added

- Add the `init` command to download and extract Liferay Portal or DXP bundles from a product ID or URL.
- Resolve partial product IDs (e.g. `dxp-2025.q4`) to the latest release by scraping the Liferay CDN, with automatic LTS suffix handling for DXP Q1 releases.
- Support standalone bundles outside a Liferay Workspace.
- Show download progress bars.
- Document installation via Homebrew and Scoop.

## [1.0.0] - 2026-03-13

### Added

- Add the `configure`, `summary`, `status`, `kill` and `reset` commands for side-by-side local Liferay instances.
- Offset Tomcat ports and set unique session cookies and HSQL databases per instance ID, editing `server.xml` and `context.xml` structurally.
- Add granular `reset` flags: `--ports`, `--props` and `--all`.
- Show the detected Liferay project type in `summary`.

### Changed

- Adopt the template v0.2.0 project discovery and Git utilities.
- Split the release workflow into separate build and release jobs.
- Track `Cargo.lock` for reliable locked CI builds.

<!-- markdownlint-disable MD049 -->
---
*Last Updated: 2026-11-11* | *Last Reviewed: 2026-11-11*
