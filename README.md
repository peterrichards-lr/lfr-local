# lfr-local

A high-performance CLI tool for managing local Liferay DXP environments. Built with Rust for speed, safety, and zero-dependency distribution.

## Key Features

- **Multi-Instance Isolation:** Offsets ports (Shutdown, HTTP, AJP, SSL) and isolates browser sessions via unique `sessionCookieName`.
- **Structural XML Editing:** Uses DOM-based parsing via `edit-xml` to ensure `server.xml` and `context.xml` updates are resilient to attribute order and preserve comments.
- **Deep Reset:** Purges OSGi state and Tomcat caches, then automatically reconstructs the directory structure to prevent JDK bind exceptions.
- **Cluster Status:** Real-time port scanning to identify which Liferay instances are currently active and their associated PIDs.

## Development

This project uses a pre-commit hook to ensure code quality. 
To enable it locally:
`cp scripts/pre-commit .git/hooks/pre-commit && chmod +x .git/hooks/pre-commit`

## Usage

Run `lfr-local` from the root of any Liferay Workspace (where the `bundles` folder is located).

### Commands

| Command | Description |
| :--- | :--- |
| `configure <ID>` | Offsets ports by ID * 100, sets unique session cookies and HSQL DBs. |
| `summary` | View all ports, Java version, product version, and DB strings at a glance. |
| `status` | Lists running instances and their PIDs. |
| `kill <ID>` | Terminates the Java process for a specific instance. |
| `reset` | Clears OSGi/Tomcat caches. Use `--ports`, `--props`, or `--all` for deeper resets. |

### Installation

```bash
cargo build --release
cp target/release/lfr-local /usr/local/bin/
```

### Configure an Instance

Prepare a Liferay bundle to run as a specific instance ID. ID `1` uses port `8180`, ID `2` uses `8280`.

```bash
lfr-local configure 1
```
