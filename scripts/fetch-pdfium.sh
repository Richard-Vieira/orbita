#!/usr/bin/env sh
# Baixa o pdfium (bblanchon/pdfium-binaries) para Linux numa versão fixa e confere o SHA-256.
# Uso: sh scripts/fetch-pdfium.sh   ->   vendor/pdfium/lib/libpdfium.so
set -eu

VERSION=7881
SHA256=1470e21b8b4a3b4ad7f85684e2da11d94f3b69a86d81dee11b9b6709d927ac1d
URL="https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F${VERSION}/pdfium-linux-x64.tgz"

ROOT=$(cd "$(dirname "$0")/.." && pwd)
DEST="$ROOT/vendor/pdfium"
[ -f "$DEST/.version-$VERSION" ] && { echo "pdfium $VERSION já está em $DEST"; exit 0; }

TMP=$(mktemp)
echo "Baixando pdfium $VERSION..."
curl -fsSL -o "$TMP" "$URL"
echo "$SHA256  $TMP" | sha256sum -c - >/dev/null || { echo "SHA-256 não confere" >&2; rm -f "$TMP"; exit 1; }

rm -rf "$DEST" && mkdir -p "$DEST"
tar -xzf "$TMP" -C "$DEST"
rm -f "$TMP"
touch "$DEST/.version-$VERSION"
echo "pdfium $VERSION pronto em $DEST/lib/libpdfium.so"
