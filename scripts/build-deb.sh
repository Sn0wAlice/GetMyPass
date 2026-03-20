#!/usr/bin/env bash
set -euo pipefail

BINARY="$1"
ARCH="$2"
VERSION="$3"

PKG_NAME="gmp"
PKG_DIR="${PKG_NAME}_${VERSION}_${ARCH}"

mkdir -p "${PKG_DIR}/DEBIAN"
mkdir -p "${PKG_DIR}/usr/bin"

cp "$BINARY" "${PKG_DIR}/usr/bin/gmp"
chmod 755 "${PKG_DIR}/usr/bin/gmp"

cat > "${PKG_DIR}/DEBIAN/control" << EOF
Package: gmp-pass
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: ${ARCH}
Maintainer: Sn0wAlice <contact@music-music.fr>
Description: Simple TUI password manager with AES-256-GCM encryption
 GetMyPass (gmp) is a fast, minimal terminal password manager.
 Stores passwords and encrypted notes in ~/.gmp/vault.enc.
 Built with Rust, Ratatui, and AES-256-GCM + Argon2id.
EOF

dpkg-deb --build "$PKG_DIR"
