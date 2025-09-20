#!/usr/bin/env bash
set -euo pipefail

# Generate a Homebrew Formula from template plus auto-computed sha256.
# Usage:
#   bash package/gen_formula.sh [--version 0.1.6] [--sha256 <sha>] [--repo https://github.com/luw2007/diff_by_time] \
#                               [--out package/Formula/dt.rb | --tap-dir /path/to/homebrew-tap]
# If --sha256 is not provided, the script downloads the release tarball to compute it.

ROOT_DIR=$(cd "$(dirname "$0")/.." && pwd)
TEMPLATE="$ROOT_DIR/package/formula_template.rb"
OUT_PATH=""
TAP_DIR=""

# Read defaults from Cargo.toml
read_kv() { sed -n "s/^$1 = \"\(.*\)\"/\1/p" "$ROOT_DIR/Cargo.toml" | head -n1; }
VERSION_DEFAULT=$(read_kv version)
DESC_DEFAULT=$(read_kv description)
LICENSE_DEFAULT=$(read_kv license)
REPO_DEFAULT=$(read_kv repository)
HOMEPAGE_DEFAULT=$(read_kv homepage)

VERSION="$VERSION_DEFAULT"
SHA256=""
REPO_URL="${REPO_DEFAULT:-https://github.com/luw2007/diff_by_time}"
HOMEPAGE="${HOMEPAGE_DEFAULT:-$REPO_URL}"
DESC="${DESC_DEFAULT:-Diff and run commands with time-based history}"
LICENSE_TXT="${LICENSE_DEFAULT:-MIT}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version|-v) VERSION="$2"; shift 2;;
    --sha256) SHA256="$2"; shift 2;;
    --repo) REPO_URL="$2"; shift 2;;
    --out) OUT_PATH="$2"; shift 2;;
    --tap-dir) TAP_DIR="$2"; shift 2;;
    --homepage) HOMEPAGE="$2"; shift 2;;
    --desc) DESC="$2"; shift 2;;
    --license) LICENSE_TXT="$2"; shift 2;;
    --help|-h)
      echo "Generate Homebrew formula from template.";
      echo "Options: --version V --sha256 S --repo URL --out PATH --tap-dir DIR --homepage URL --desc TEXT";
      exit 0;;
    *) echo "Unknown arg: $1"; exit 1;;
  esac
done

URL="$REPO_URL/archive/refs/tags/v${VERSION}.tar.gz"

if [[ -z "$SHA256" ]]; then
  TMP_TARBALL=$(mktemp -t dt-src-XXXXXX.tar.gz)
  echo "Downloading $URL ..."
  if curl -L -f -sS "$URL" -o "$TMP_TARBALL"; then
    echo "Computing sha256 ..."
    if command -v shasum >/dev/null 2>&1; then
      SHA256=$(shasum -a 256 "$TMP_TARBALL" | awk '{print $1}')
    else
      SHA256=$(sha256sum "$TMP_TARBALL" | awk '{print $1}')
    fi
    rm -f "$TMP_TARBALL"
  else
    echo "WARN: failed to download tarball (tag may not exist yet); leaving sha256 placeholder" >&2
    SHA256="REPLACE_WITH_TARBALL_SHA256"
  fi
fi

esc_sed() { printf '%s' "$1" | sed -e 's/[\/&]/\\&/g'; }

render() {
  sed -e "s/{{URL}}/$(esc_sed "$URL")/g" \
      -e "s/{{SHA256}}/$(esc_sed "$SHA256")/g" \
      -e "s/{{DESC}}/$(esc_sed "$DESC")/g" \
      -e "s/{{HOMEPAGE}}/$(esc_sed "$HOMEPAGE")/g" \
      -e "s/{{LICENSE}}/$(esc_sed "$LICENSE_TXT")/g" \
      "$TEMPLATE"
}

if [[ -n "$TAP_DIR" ]]; then
  OUT_PATH="$TAP_DIR/Formula/dt.rb"
fi

if [[ -z "$OUT_PATH" ]]; then
  mkdir -p "$ROOT_DIR/package/Formula"
  OUT_PATH="$ROOT_DIR/package/Formula/dt.rb"
fi

mkdir -p "$(dirname "$OUT_PATH")"
render > "$OUT_PATH"
echo "Wrote formula: $OUT_PATH"
echo "url: $URL"
echo "sha256: $SHA256"
