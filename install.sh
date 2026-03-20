#!/bin/bash
# Install eto CLI
set -euo pipefail

REPO="etofdn/eto-cli"
VERSION="${1:-latest}"

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
  darwin)
    case "$ARCH" in
      arm64|aarch64) ASSET="eto-macos-arm64" ;;
      x86_64)        ASSET="eto-macos-x86_64" ;;
      *) echo "Unsupported arch: $ARCH"; exit 1 ;;
    esac
    ;;
  linux)
    ASSET="eto-linux-x86_64"
    ;;
  *) echo "Unsupported OS: $OS"; exit 1 ;;
esac

if [ "$VERSION" = "latest" ]; then
  URL="https://github.com/$REPO/releases/latest/download/${ASSET}.tar.gz"
else
  URL="https://github.com/$REPO/releases/download/$VERSION/${ASSET}.tar.gz"
fi

echo "Installing eto CLI ($ASSET)..."
TMPDIR=$(mktemp -d)
curl -sL "$URL" | tar xz -C "$TMPDIR"
sudo mv "$TMPDIR/eto" /usr/local/bin/eto
rm -rf "$TMPDIR"
echo "Installed: $(eto --version 2>/dev/null || echo 'eto')"
echo ""
echo "Get started:"
echo "  eto config set --url http://98.93.46.219:8899"
echo "  eto airdrop 10"
echo "  eto balance"
