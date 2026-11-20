#!/usr/bin/env sh
set -eu

CHANNEL="${APHRODY_CHANNEL:-${ZED_CHANNEL:-stable}}"
VERSION="${APHRODY_VERSION:-${ZED_VERSION:-latest}}"
BASE_URL="${APHRODY_RELEASES_URL:-https://releases.aphrody.com}"
BUNDLE="${APHRODY_BUNDLE_PATH:-${ZED_BUNDLE_PATH:-}}"

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

if [ -n "$BUNDLE" ]; then
  echo "Installing Aphrody from local bundle $BUNDLE..."
  ASSET="$(basename "$BUNDLE")"
  cp "$BUNDLE" "$TMP/$ASSET"
else
  echo "Downloading Aphrody ($CHANNEL/$VERSION) for $OS/$ARCH from $URL..."
  curl -fsSL "$URL" -o "$TMP/$ASSET"
fi

mkdir -p "$HOME/.local/bin" "$HOME/.local/share/aphrody"
if [ "$OS" = "linux" ]; then
  APP_DIR="$(tar -tzf "$TMP/$ASSET" | sed -n 's|^\([^/]*\.app\)/.*|\1|p' | head -n 1)"
  if [ -n "$APP_DIR" ]; then
    # script/bundle-linux layout: <name>.app/{bin,libexec,share}
    rm -rf "$HOME/.local/$APP_DIR"
    tar -xzf "$TMP/$ASSET" -C "$HOME/.local"
    for bin in aphrody zed; do
      if [ -x "$HOME/.local/$APP_DIR/bin/$bin" ]; then
        ln -sfn "$HOME/.local/$APP_DIR/bin/$bin" "$HOME/.local/bin/aphrody"
        break
      fi
    done
  else
    tar -xzf "$TMP/$ASSET" -C "$HOME/.local/share/aphrody"
    chmod +x "$HOME/.local/share/aphrody/aphrody"
    ln -sfn "$HOME/.local/share/aphrody/aphrody" "$HOME/.local/bin/aphrody"
  fi
  echo "Installed aphrody -> $HOME/.local/bin/aphrody"
else
  echo "Downloaded macOS image -> $TMP/$ASSET"
fi
