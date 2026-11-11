# lfr-local

[![GitHub Release](https://img.shields.io/github/v/release/peterrichards-lr/lfr-local)](https://github.com/peterrichards-lr/lfr-local/releases)
[![License](https://img.shields.io/github/license/peterrichards-lr/lfr-local)](LICENSE)
[![Rust CI](https://github.com/peterrichards-lr/lfr-local/actions/workflows/rust.yml/badge.svg)](https://github.com/peterrichards-lr/lfr-local/actions/workflows/rust.yml)

`lfr-local` is a Liferay Local Instance Manager: a fast, single-binary CLI for running several local Liferay DXP or Portal bundles side by side. It downloads and initializes bundles, offsets each instance's ports and session cookie so they don't collide, and helps you inspect, stop and reset them. It is written in Rust.

## Features

- **Multi-Instance Isolation:** Offsets ports (Shutdown, HTTP, AJP, SSL redirect) by instance ID and isolates browser sessions via a unique `sessionCookieName` and HSQL database per instance.
- **Standalone & Workspace Support:** Works within a Liferay Workspace or as a standalone tool for individual bundles.
- **Automated Bundle Initialization:** Downloads and extracts Portal or DXP bundles directly from Liferay's CDN or a custom mirror.
- **Blade Cache Reuse:** Reuses bundles already in the Liferay Blade cache (`~/.liferay/bundles`), including `.zip`, `.tar.gz` and `.7z` archives, before downloading.
- **Smart Version Resolution:** Resolves partial product IDs to the latest available update (e.g. `dxp-2024.q1` -> `2024.q1.x-lts`).
- **Structural XML Editing:** Uses DOM-based parsing via `edit-xml` so `server.xml` and `context.xml` updates are resilient to attribute order.
- **Deep Reset:** Purges OSGi state and Tomcat caches, then recreates the Tomcat `temp` and `work` directories.
- **Instance Status:** Port scanning to show which instances are running.

## Installation

### Homebrew (macOS / Linux)

```bash
brew tap peterrichards-lr/tap
brew install lfr-local
```

The formula builds from source, so Homebrew installs Rust as a build dependency.

### Scoop (Windows)

```bash
scoop bucket add peterrichards-lr https://github.com/peterrichards-lr/scoop-bucket
scoop install lfr-local
```

### WSL

Install [Homebrew on Linux](https://docs.brew.sh/Homebrew-on-Linux) inside your WSL distribution, then use the Homebrew commands above.

### Manual Download

Download a pre-built binary for your platform from the [Releases page](https://github.com/peterrichards-lr/lfr-local/releases) and place it on your `PATH`.

### Build from Source

Ensure you have Rust and Cargo installed, then run:

```bash
cargo install --path .
```

## Usage

Run `lfr-local` from the root of a Liferay Workspace or from within a standalone Liferay bundle directory.

```bash
lfr-local --help
lfr-local <command> --help
lfr-local --version
```

### Commands

| Command | Description |
| :--- | :--- |
| `init` | Downloads and initializes a new Liferay bundle from a product ID or URL. |
| `configure <ID>` | Offsets ports by ID * 100, sets a unique session cookie and HSQL database. |
| `summary` | Shows project type, Liferay home, Java version, product version, ports, session cookie and database URL. |
| `status [ID]` | Shows which instances are running (IDs 0-5 by default, or a single ID). |
| `kill <ID>` | Terminates the Java process for a specific instance. |
| `reset` | Clears OSGi state and Tomcat caches. Use `--ports`, `--props`, or `--all` for deeper resets. |

### Initialize a New Liferay Bundle

Download and extract a Liferay Portal or DXP bundle into a new directory (`--name`) under the current directory.

| Option | Description |
| :--- | :--- |
| `-p, --product <PRODUCT>` | Product ID with a `portal-` or `dxp-` prefix (e.g. `portal-7.4.3`, `dxp-2024.q1`). |
| `-u, --url <URL>` | Direct URL to a bundle `.zip` file. |
| `--base-url <BASE_URL>` | Base URL for product resolution (overrides the Liferay CDN). |
| `-n, --name <NAME>` | Target directory name for the bundle (required). |

Provide either `--product` or `--url`. The tool resolves partial product IDs to the **latest available update** by scraping the Liferay CDN, and adds the **LTS** suffix for DXP Q1 releases automatically. If a matching bundle is already in the Blade cache (`~/.liferay/bundles`), it is extracted from there instead of being downloaded. If the target directory exists, you are asked before it is overwritten.

```bash
# Initialize the latest 7.4.3.x Portal version
lfr-local init --product portal-7.4.3 --name my-portal

# Initialize the latest 2024 Q1 (LTS) update
# Resolves to: liferay-dxp-tomcat-2024.q1.x-lts.zip
lfr-local init --product dxp-2024.q1 --name my-dxp-lts

# Use a direct URL to a .zip bundle
lfr-local init --url https://.../bundle.zip --name my-custom-bundle

# Override the base CDN (e.g., for an internal mirror)
lfr-local init --product dxp-2024.q1 --name my-dxp --base-url https://my-mirror.com/dxp/
```

### Configure an Instance

Prepare a Liferay bundle to run as a specific instance ID. Ports are offset by `ID * 100`: ID `1` uses HTTP port `8180`, ID `2` uses `8280`.

| Option | Description |
| :--- | :--- |
| `<INSTANCE_ID>` | Unique instance ID used to derive ports and the session cookie. |
| `-w, --workspace-path <PATH>` | Path to the Liferay Workspace (defaults to auto-detection from the current directory). |
| `-d, --db-name <NAME>` | Custom HSQL database name (defaults to `lportal_<ID>`). |
| `--clear-data` | Wipe persistent data (the bundle's `data` directory: Elasticsearch indexes and HSQL). |

```bash
lfr-local configure 1
```

This updates `conf/server.xml` (shutdown, HTTP, AJP and redirect ports), sets `sessionCookieName` in `conf/context.xml`, and writes `session.cookie.name=LFR_SESSION_<ID>` and an HSQL `jdbc.default.url` to `portal-ext.properties`.

### Check Status and Stop an Instance

```bash
# Check instances 0-5 (HTTP ports 8080-8580)
lfr-local status

# Check a single instance
lfr-local status 1

# Stop instance 1
lfr-local kill 1
```

`status` and `kill` detect a running instance by checking whether its HTTP port is open. The PID shown, and the process `kill` terminates, is the first Java process found on the machine, so take care when other Java processes are running.

### Reset an Instance

| Option | Description |
| :--- | :--- |
| `-w, --workspace-path <PATH>` | Path to the Liferay Workspace. |
| `--ports` | Reset `server.xml` ports to the defaults (8080, 8005, 8009, 8443). |
| `--props` | Remove the session cookie and database URL from `portal-ext.properties`. |
| `--all` | Also wipe all persistent data (databases and indexes), after confirmation. |

```bash
lfr-local reset
lfr-local reset --ports --props
lfr-local reset --all
```

## Development

Activate the shared pre-commit hook, which runs `scripts/doctor.py`, `cargo fmt` and `cargo clippy`:

```bash
git config core.hooksPath .githooks
chmod +x .githooks/pre-commit
```

Useful Cargo aliases are defined in `.cargo/config.toml`: `cargo lint` and `cargo fmt-check`.

## Releasing

1. Bump `version` in `Cargo.toml` and run `cargo build` to update `Cargo.lock`.
2. Move the `[Unreleased]` entries in `CHANGELOG.md` into a new `## [x.y.z] - YYYY-MM-DD` section.
3. Commit, then create an annotated tag and push it:
   ```bash
   python3 scripts/release.py --tag vX.Y.Z
   git push origin vX.Y.Z
   ```
   The release workflow fails if the tag isn't annotated, doesn't match `Cargo.toml`, or has no `CHANGELOG.md` section.
4. The Homebrew tap and Scoop bucket pick up the new release automatically within a couple of hours. `.gemini/prompts/update-distribution-channels.md` is only needed for a tool's first release.

## Disclaimer

**This tool is provided "as-is", without warranty of any kind. Several commands are destructive: `init` can overwrite an existing directory, `configure` and `reset` rewrite Tomcat and portal configuration files, `configure --clear-data` and `reset --all` delete Liferay data, and `kill` terminates a Java process. Back up anything you need before use. You use this tool at your own risk.**

## License

This project is licensed under the [MIT License](LICENSE).
