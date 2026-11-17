#!/usr/bin/env sh
set -eu

CHANNEL="${APHRODY_CHANNEL:-${ZED_CHANNEL:-stable}}"
VERSION="${APHRODY_VERSION:-${ZED_VERSION:-latest}}"
BASE_URL="${APHRODY_RELEASES_URL:-https://releases.aphrody.com}"

OS="$(uname -s)"
ARCH="$(uname -m)"
case "$OS" in
  Darwin) OS="macos" ;;
  Linux) OS="linux" ;;
  *) echo "Unsupported OS: $OS" >&2; exit 1 ;;
esac

case "$ARCH" in
  arm64|aarch64) ARCH="aarch64" ;;
  x86_64|amd64) ARCH="x86_64" ;;
  *) echo "Unsupported architecture: $ARCH" >&2; exit 1 ;;
esac

if [ "$OS" = "macos" ]; then
  ASSET="Aphrody-$([ "$ARCH" = "aarch64" ] && echo arm64 || echo x64).dmg"
else
  ASSET="aphrody-$ARCH.tar.gz"
fi

URL="$BASE_URL/assets/$CHANNEL/$VERSION/$ASSET"
TMP="$(mktemp -d "${TMPDIR:-/tmp}/aphrody-install-XXXXXX")"
trap 'rm -rf "$TMP"' EXIT INT TERM

echo "Downloading Aphrody ($CHANNEL/$VERSION) for $OS/$ARCH from $URL..."
curl -fsSL "$URL" -o "$TMP/$ASSET"

mkdir -p "$HOME/.local/bin" "$HOME/.local/share/aphrody"
if [ "$OS" = "linux" ]; then
  tar -xzf "$TMP/$ASSET" -C "$HOME/.local/share/aphrody"
  chmod +x "$HOME/.local/share/aphrody/aphrody"
  ln -sfn "$HOME/.local/share/aphrody/aphrody" "$HOME/.local/bin/aphrody"
  echo "Installed aphrody -> $HOME/.local/bin/aphrody"
else
  echo "Downloaded macOS image -> $TMP/$ASSET"
fi
