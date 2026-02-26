# lockpick

> Blazing-fast JS/TS dependency analyzer CLI, built with Rust.

Analyze your pnpm project's dependencies in milliseconds — detect unused packages and scan for known vulnerabilities.

## Features

- **Unused dependency detection** — Parses JS/TS source files with [oxc](https://oxc.rs) to find packages you declared but never imported
- **Vulnerability scanning** — Queries [OSV.dev](https://osv.dev) for known CVEs across all your dependencies
- **Fast** — Native Rust binary, no Node.js runtime needed
- **Bilingual** — English and Chinese output (`--lang zh`)
- **Multiple output formats** — Terminal (colored tables) or JSON

## Installation

```bash
cargo install lockpick
```

Or build from source:

```bash
git clone https://github.com/Dean0801/lockpick.git
cd lockpick
cargo build --release
```

## Usage

```bash
# Full scan (unused deps + vulnerability audit)
lockpick

# Scan a specific project
lockpick --path /path/to/project

# Unused dependencies only
lockpick unused

# Vulnerability audit only
lockpick audit

# Chinese output
lockpick --lang zh

# JSON output
lockpick --format json

# Skip devDependencies
lockpick --no-dev

# Ignore specific packages
lockpick --ignore react --ignore lodash
```

## Supported Lockfiles

| Lockfile | Status |
|----------|--------|
| pnpm-lock.yaml (v9) | ✅ Supported |
| package-lock.json | 🔜 Planned |
| yarn.lock | 🔜 Planned |

## How It Works

1. Parse `pnpm-lock.yaml` to build a dependency graph
2. Scan JS/TS source files using [oxc_parser](https://crates.io/crates/oxc_parser) to extract imports
3. Compare declared dependencies vs actual imports to find unused packages
4. Query [OSV.dev](https://osv.dev) batch API for known vulnerabilities
5. Output results as colored terminal tables or JSON

## Environment Variables

| Variable | Description |
|----------|-------------|
| `LOCKPICK_LANG` | Set default language (`en` or `zh`) |
| `LANG` / `LC_ALL` | System locale fallback for language detection |

## License

MIT
