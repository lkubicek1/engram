#!/bin/sh
set -eu

ENGRAM_REPO="${ENGRAM_REPO:-lkubicek1/engram}"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
ENGRAM_VERSION="${ENGRAM_VERSION:-latest}"

# Detect platform
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
  linux) OS="linux" ;;
  darwin) OS="darwin" ;;
  *) echo "Unsupported OS: $OS" >&2; exit 1 ;;
esac

case "$ARCH" in
  x86_64) ARCH="x86_64" ;;
  aarch64|arm64) ARCH="aarch64" ;;
  *) echo "Unsupported architecture: $ARCH" >&2; exit 1 ;;
esac

# Keep this list in sync with the release workflow.
if [ "$OS" = "linux" ] && [ "$ARCH" = "aarch64" ]; then
  echo "Unsupported platform for this Engram release: linux-aarch64" >&2
  echo "Supported: linux-x86_64, darwin-x86_64, darwin-aarch64" >&2
  exit 1
fi

ASSET="engram-${OS}-${ARCH}"

if [ "$ENGRAM_VERSION" = "latest" ]; then
  BASE_URL="https://github.com/${ENGRAM_REPO}/releases/latest/download"
else
  BASE_URL="https://github.com/${ENGRAM_REPO}/releases/download/v${ENGRAM_VERSION}"
fi

CHECKSUM_URL="${BASE_URL}/checksums.txt"
BIN_URL="${BASE_URL}/${ASSET}"

TMP_DIR=$(mktemp -d 2>/dev/null || mktemp -d -t engram)
cleanup() { rm -rf "$TMP_DIR"; }
trap cleanup EXIT INT TERM

CHECKSUM_FILE="$TMP_DIR/checksums.txt"
TMP_BIN="$TMP_DIR/$ASSET"

if command -v curl >/dev/null 2>&1; then
  curl -fsSL "$CHECKSUM_URL" -o "$CHECKSUM_FILE"
  curl -fsSL "$BIN_URL" -o "$TMP_BIN"
elif command -v wget >/dev/null 2>&1; then
  wget -q "$CHECKSUM_URL" -O "$CHECKSUM_FILE"
  wget -q "$BIN_URL" -O "$TMP_BIN"
else
  echo "Error: curl or wget is required" >&2
  exit 1
fi

expected=$(grep "  $ASSET\$" "$CHECKSUM_FILE" | awk '{print $1}' | head -n 1)
if [ -z "$expected" ]; then
  echo "Error: checksum entry for $ASSET not found" >&2
  exit 1
fi

if command -v sha256sum >/dev/null 2>&1; then
  actual=$(sha256sum "$TMP_BIN" | awk '{print $1}')
elif command -v shasum >/dev/null 2>&1; then
  actual=$(shasum -a 256 "$TMP_BIN" | awk '{print $1}')
else
  echo "Error: sha256sum or shasum is required for checksum verification" >&2
  exit 1
fi

if [ "$expected" != "$actual" ]; then
  echo "Error: checksum mismatch for $ASSET" >&2
  echo "Expected: $expected" >&2
  echo "Actual:   $actual" >&2
  exit 1
fi

mkdir -p "$INSTALL_DIR"
INSTALL_PATH="$INSTALL_DIR/engram"

chmod +x "$TMP_BIN"
mv "$TMP_BIN" "$INSTALL_PATH"

echo "Installed engram to: $INSTALL_PATH" >&2
echo "Try: engram --version" >&2

echo "If 'engram' is not found, add $INSTALL_DIR to your PATH." >&2
