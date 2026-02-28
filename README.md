# lockpick

> Blazing-fast JS/TS dependency analyzer CLI, built with Rust.

Analyze your JS/TS project's dependencies in milliseconds — detect unused packages, scan for vulnerabilities, find duplicates, and measure dependency sizes.

## Features

- **Unused dependency detection** — Parses JS/TS source files with [oxc](https://oxc.rs) to find packages you declared but never imported
- **Config file awareness** — Scans ESLint, Babel, PostCSS, Vite, Next.js, Webpack, Tailwind config files to detect plugin references (supports JSONC comments)
- **Scripts awareness** — Parses `package.json` scripts to detect CLI tools (e.g. `tsc` → `typescript`), supports chained commands (`&&`, `||`, `;`, `|`)
- **Monorepo support** — Detects pnpm/npm/yarn workspaces and analyzes each package independently
- **Project config (.lockpickrc)** — JSON/YAML config file for persistent ignore rules, language, and extra config paths
- **Vulnerability scanning** — Queries [OSV.dev](https://osv.dev) for known CVEs, computes CVSS 3.x Base Score from vector strings, with local file cache and progress bar
- **Duplicate detection** — Finds packages with multiple versions installed in your lockfile
- **Size analysis** — Measures the disk size of each dependency in `node_modules`
- **License compliance** — Extracts license info from `node_modules`, normalizes SPDX aliases, supports allow/deny policy via `.lockpickrc`
- **Auto-fix** — `lockpick-cli fix` removes unused dependencies via your package manager, supports monorepo workspaces and `--dry-run`
- **Outdated detection** — `lockpick-cli outdated` checks npm registry for newer versions with progress bar, correlates with vulnerability data for upgrade priority
- **Supply chain security** — `lockpick-cli supply-chain` detects typosquatting, scope confusion, and version anomaly attacks; High/Critical risks affect exit code
- **Multi-lockfile support** — Auto-detects pnpm-lock.yaml, bun.lock, package-lock.json, and yarn.lock (including yarn Berry v2/v3/v4)
- **ESM + CJS + dynamic import** — Handles `import`, `require()`, `require.resolve()`, and `import()` syntax with deep AST traversal (if/try/class/arrow functions)
- **CI-friendly** — Exits with code 1 when unused deps or vulnerabilities are found; supports `--fail-on` threshold and `.lockpickrc` thresholds for fine-grained CI gating
- **Smart @types association** — `@types/react` won't be flagged as unused if `react` is imported
- **Dependency tree** — `lockpick tree` visualizes the full dependency graph (terminal, DOT, JSON, Mermaid), with `--focus` and `--depth`
- **Diff comparison** — `lockpick diff <baseline.json>` compares current state against a baseline, showing new and resolved issues
- **Fast** — Native Rust binary, no Node.js runtime needed
- **Bilingual** — English and Chinese output (`--lang zh`)
- **Multiple output formats** — Terminal (colored tables), JSON, or Markdown (`--output <file>` to write to file)

## Installation

### npm / pnpm / yarn

```bash
npm install -D lockpick-cli
pnpm add -D lockpick-cli
yarn add -D lockpick-cli

# Or run directly
npx lockpick-cli
```

### Build from source

```bash
git clone https://github.com/Dean0801/lockpick.git
cd lockpick
cargo build --release
```

## Usage

```bash
# Full scan (unused deps + vulnerability audit)
lockpick-cli

# Scan a specific project
lockpick-cli --path /path/to/project

# Unused dependencies only
lockpick-cli unused

# Vulnerability audit only
lockpick-cli audit

# Chinese output
lockpick-cli --lang zh

# JSON output
lockpick-cli --format json

# Skip devDependencies
lockpick-cli --no-dev

# Ignore specific packages
lockpick-cli --ignore react --ignore lodash

# Auto-remove unused dependencies
lockpick-cli fix

# Dry run (preview what would be removed)
lockpick-cli fix --dry-run

# Disable vulnerability cache
lockpick-cli audit --no-cache

# Markdown report to file
lockpick-cli --format markdown --output report.md

# Dependency tree visualization
lockpick-cli tree
lockpick-cli tree --format dot          # Graphviz DOT
lockpick-cli tree --format mermaid      # Mermaid diagram
lockpick-cli tree --focus react         # Focus on a package
lockpick-cli tree --depth 2             # Limit depth

# Diff against baseline
lockpick-cli --format json --output baseline.json   # Save baseline
lockpick-cli diff baseline.json                      # Compare later
lockpick-cli diff baseline.json --format markdown    # Markdown diff

# Outdated dependency check
lockpick-cli outdated
lockpick-cli outdated --level patch        # Filter by semver level
lockpick-cli outdated --no-audit           # Skip vulnerability correlation
lockpick-cli outdated --registry https://registry.npmmirror.com  # Custom registry

# Supply chain security analysis
lockpick-cli supply-chain

# CI threshold gate
lockpick-cli --fail-on critical         # Fail on critical vulns only
lockpick-cli --fail-on any              # Fail on any issue
```

## Supported Lockfiles

| Lockfile | Status |
|----------|--------|
| pnpm-lock.yaml (v9) | ✅ Supported |
| package-lock.json (v1/v2/v3) | ✅ Supported |
| yarn.lock (v1 + Berry v2/v3/v4) | ✅ Supported |
| bun.lock | ✅ Supported |

## Configuration (.lockpickrc)

Create a `.lockpickrc.json` or `.lockpickrc.yaml` in your project root:

```json
{
  "ignore": ["husky", "lint-staged"],
  "skip_dev": false,
  "lang": "zh",
  "extra_configs": ["jest.config.ts"],
  "license": {
    "allow": ["MIT", "ISC", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause"],
    "deny": ["GPL-3.0"]
  },
  "cache_ttl": 7200,
  "registry": "https://registry.npmjs.org",
  "thresholds": {
    "max_critical": 0,
    "max_high": 5,
    "max_unused": 10,
    "max_duplicates": -1,
    "fail_on_license": true,
    "max_supply_chain_high": 0
  }
}
```

CLI arguments override config file settings.

## How It Works

1. Load `.lockpickrc` config (if present) and merge with CLI args
2. Auto-detect and parse lockfile (pnpm-lock.yaml / package-lock.json / yarn.lock / bun.lock)
3. Detect monorepo workspaces (pnpm/npm/yarn) — analyze each package independently
4. Scan JS/TS source files using [oxc_parser](https://crates.io/crates/oxc_parser) to extract imports (`import`, `require()`, `import()`)
5. Scan config files (ESLint, Vite, Babel, PostCSS, etc.) for plugin references
6. Scan `package.json` scripts for CLI tool usage
7. Compare declared dependencies vs actual usage to find unused packages
8. Detect duplicate packages with multiple versions in the lockfile
9. Measure dependency sizes in `node_modules`
10. Extract license info and check against allow/deny policy
11. Query [OSV.dev](https://osv.dev) batch API for known vulnerabilities (with local file cache)
12. Check npm registry for outdated dependencies and compute upgrade priority
13. Run supply chain security checks (typosquatting, scope confusion, version anomaly)
14. Output results as colored terminal tables, JSON, or Markdown

## Environment Variables

| Variable | Description |
|----------|-------------|
| `LOCKPICK_LANG` | Set default language (`en` or `zh`) |
| `LANG` / `LC_ALL` | System locale fallback for language detection |

## License

MIT
