#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
NPM_DIR="$REPO_ROOT/npm"

# Read version from Cargo.toml
VERSION=$(grep '^version' "$REPO_ROOT/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')
echo "Version: $VERSION"

# Update all package.json versions
for pkg in "$NPM_DIR"/lockpick-cli*/package.json; do
  sed -i.bak "s/\"version\": \".*\"/\"version\": \"$VERSION\"/" "$pkg"
  rm -f "$pkg.bak"
done

# Update optionalDependencies versions in main package
MAIN_PKG="$NPM_DIR/lockpick-cli/package.json"
sed -i.bak "s/\"lockpick-cli-\([^\"]*\)\": \"[^\"]*\"/\"lockpick-cli-\1\": \"$VERSION\"/g" "$MAIN_PKG"
rm -f "$MAIN_PKG.bak"

echo "All package.json updated to $VERSION"

# Platform mapping: directory -> rust target -> binary name
declare -A TARGETS=(
  ["lockpick-cli-darwin-arm64"]="aarch64-apple-darwin"
  ["lockpick-cli-darwin-x64"]="x86_64-apple-darwin"
  ["lockpick-cli-linux-x64-gnu"]="x86_64-unknown-linux-gnu"
  ["lockpick-cli-linux-x64-musl"]="x86_64-unknown-linux-musl"
  ["lockpick-cli-linux-arm64-gnu"]="aarch64-unknown-linux-gnu"
  ["lockpick-cli-win32-x64"]="x86_64-pc-windows-msvc"
)

ARTIFACTS_DIR="${ARTIFACTS_DIR:-$REPO_ROOT/artifacts}"

for pkg in "${!TARGETS[@]}"; do
  target="${TARGETS[$pkg]}"
  dest="$NPM_DIR/$pkg"

  if [[ "$pkg" == *"win32"* ]]; then
    bin="lockpick.exe"
  else
    bin="lockpick"
  fi

  src="$ARTIFACTS_DIR/$target/$bin"
  if [[ -f "$src" ]]; then
    cp "$src" "$dest/$bin"
    chmod +x "$dest/$bin"
    echo "Copied $bin -> $pkg"
  else
    echo "Warning: $src not found, skipping $pkg"
  fi
done

echo "Done!"
