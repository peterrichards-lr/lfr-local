# lfr-local

A high-performance CLI tool for managing local Liferay DXP environments. Built with Rust for speed, safety, and zero-dependency distribution.

## Key Features

- **Multi-Instance Isolation:** Offsets ports (Shutdown, HTTP, AJP, SSL) and isolates browser sessions via unique `sessionCookieName`.
- **Standalone & Workspace Support:** Works seamlessly within a Liferay Workspace or as a standalone tool for individual bundles.
- **Automated Bundle Initialization:** Downloads and extracts Portal or DXP bundles directly from Liferay's CDN or custom mirrors.
- **Smart Version Resolution:** Automatically identifies the latest update version from product prefixes (e.g., `dxp-2024.q1` -> `2024.q1.0-lts`).
- **Structural XML Editing:** Uses DOM-based parsing via `edit-xml` to ensure `server.xml` and `context.xml` updates are resilient to attribute order and preserve comments.
- **Deep Reset:** Purges OSGi state and Tomcat caches, then automatically reconstructs the directory structure to prevent JDK bind exceptions.
- **Cluster Status:** Real-time port scanning to identify which Liferay instances are currently active and their associated PIDs.

## Development

This project uses a pre-commit hook to ensure code quality. 
To enable it locally:
`cp scripts/pre-commit .git/hooks/pre-commit && chmod +x .git/hooks/pre-commit`

## Installation

### From Source
Ensure you have Rust and Cargo installed, then run:
```bash
cargo install --path .
```

### macOS & Linux (Homebrew)
Install via the Liferay Homebrew tap:
```bash
brew tap peterrichards-lr/tap
brew install lfr-local
```

### Windows (Scoop)
Install via the Liferay Scoop bucket:
```bash
scoop bucket add lfr-local https://github.com/peterrichards-lr/scoop-bucket
scoop install lfr-local
```

## Usage

Run `lfr-local` from the root of any Liferay Workspace or from within a standalone Liferay bundle directory.

### Commands

| Command | Description |
| :--- | :--- |
| `init` | Downloads and initializes a new Liferay bundle from a product ID or URL. |
| `configure <ID>` | Offsets ports by ID * 100, sets unique session cookies and HSQL DBs. |
| `summary` | View all ports, Java version, product version, and DB strings at a glance. |
| `status` | Lists running instances and their PIDs. |
| `kill <ID>` | Terminates the Java process for a specific instance. |
| `reset` | Clears OSGi/Tomcat caches. Use `--ports`, `--props`, or `--all` for deeper resets. |

### Initialize a new Liferay Bundle

Download and extract a Liferay Portal or DXP bundle into a specific directory. 

The tool will automatically resolve partial product IDs to the **latest available update** by scraping the Liferay CDN. It also handles **LTS** suffixes for DXP Q1 releases automatically.

```bash
# Initialize the latest 7.4.3.x Portal version
lfr-local init --product portal-7.4.3 --name my-portal

# Initialize the latest 2024 Q1 (LTS) update
# Resolves to: liferay-dxp-tomcat-2024.q1.x-lts.zip
lfr-local init --product dxp-2024.q1 --name my-dxp-lts

# Use a direct URL
lfr-local init --url https://.../bundle.zip --name my-custom-bundle

# Override the base CDN (e.g., for an internal mirror)
lfr-local init --product dxp-2024.q1 --name my-dxp --base-url https://my-mirror.com/dxp/
```

### Configure an Instance

Prepare a Liferay bundle to run as a specific instance ID. ID `1` uses port `8180`, ID `2` uses `8280`.

```bash
lfr-local configure 1
```
