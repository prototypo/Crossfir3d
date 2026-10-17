#!/bin/sh
# Build and run a local Crossfire C server for MacCrossfire development.
#   scripts/server.sh build   clone pinned upstream repos, build, install into upstream/install
#   scripts/server.sh run     run the installed server in the foreground (port 13327)
# Built with autotools (works on macOS with the workarounds below); the in-tree CMake build
# does not install arch/maps data, so it is not used.
set -eu

ROOT=$(cd "$(dirname "$0")/.." && pwd)
UP="$ROOT/upstream"
PREFIX="$UP/install"
SF=https://git.code.sf.net/p/crossfire

SERVER_PIN=2f5be027b95a5d9b21c3038b677434963d000fe7
ARCH_PIN=b14b3f7e72e40bd6ac871e317e85aa6defc7ccff
MAPS_PIN=479644efa97b6d4469c3c751f20d374e71fe4d42

fetch() { # repo pin
    [ -d "$UP/$1" ] || git clone "$SF/$1" "$UP/$1"
    git -C "$UP/$1" checkout -q "$2"
}

build() {
    for dep in autoconf automake libtool pkg-config flex sqlite; do
        brew list "$dep" >/dev/null 2>&1 || brew install "$dep"
    done
    mkdir -p "$UP"
    fetch crossfire-server "$SERVER_PIN"
    fetch crossfire-arch "$ARCH_PIN"
    fetch crossfire-maps "$MAPS_PIN"
    ln -sfn "$UP/crossfire-arch" "$UP/crossfire-server/lib/arch"
    ln -sfn "$UP/crossfire-maps" "$UP/crossfire-server/lib/maps"

    # Python modules the map scripts import. The server embeds Python, so they go in a
    # plain folder put on PYTHONPATH by `run`. python-cjson (in the maps' requirements)
    # is Python 2 only, so a stand-in over the stdlib json module replaces it.
    mkdir -p "$UP/pylib"
    python3 -m pip install -q --target "$UP/pylib" requests
    printf 'from json import loads as decode, dumps as encode\n' > "$UP/pylib/cjson.py"

    BREW=$(brew --prefix)
    cd "$UP/crossfire-server"
    export PATH="$BREW/opt/flex/bin:$BREW/opt/libtool/libexec/gnubin:$PATH"
    export ACLOCAL_PATH="$BREW/share/aclocal"
    export PKG_CONFIG_PATH="$BREW/opt/sqlite/lib/pkgconfig:${PKG_CONFIG_PATH:-}"
    # macOS has no <malloc.h>; server/races.cpp includes it.
    mkdir -p "$UP/compat"
    printf '#include <stdlib.h>\n' > "$UP/compat/malloc.h"
    export CPPFLAGS="-I$UP/compat ${CPPFLAGS:-}"
    # macOS is missing from the socket-option platform list in socket/init.cpp, so the
    # IPv6 listener also grabs IPv4 and the IPv4 bind is retried forever (log spam).
    grep -q '__APPLE__) ||' socket/init.cpp ||
        sed -i '' 's/defined(__FreeBSD__) || defined(__OpenBSD__) ||/defined(__FreeBSD__) || defined(__OpenBSD__) || defined(__APPLE__) ||/' socket/init.cpp
    # No metaserver: a dev server must not be listed publicly, and its thread code is Linux-only.
    sh ./autogen.sh --prefix="$PREFIX" --without-libcurl
    make -j"$(sysctl -n hw.ncpu)"
    make install
    ln -sfn "$UP/crossfire-maps" "$PREFIX/share/crossfire/maps"
}

run() {
    PYTHONPATH="$UP/pylib" exec "$PREFIX/bin/crossfire-server" "$@"
}

case "${1:-}" in
    build) build ;;
    run) shift; run "$@" ;;
    *) echo "usage: $0 build|run" >&2; exit 1 ;;
esac
