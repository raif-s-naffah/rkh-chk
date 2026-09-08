#!/usr/bin/bash

# SPDX-License-Identifier: GPL-3.0-or-later

echo "🔔 Build (release)..."
cargo b -r

if command -v upx >/dev/null 2>&1; then
    echo "🔔 Compress (upx) binary..."
    rm target/release/rkh-chk.upx || true
    upx --best --lzma -o target/release/rkh-chk.upx target/release/rkh-chk
    # always ensure final executable is called 'rkh-chk'
    rm target/release/rkh-chk.orig || true
    mv target/release/rkh-chk target/release/rkh-chk.orig
    mv target/release/rkh-chk.upx target/release/rkh-chk
else
    echo "🔔 Could not find 'upx'. Continue uncompressed..."
fi

echo "🔔 Generate RPM..."
cargo generate-rpm
