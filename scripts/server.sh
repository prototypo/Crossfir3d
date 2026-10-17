#!/bin/sh
# Build and run a local Crossfire C server for MacCrossfire development.
#   scripts/server.sh build   clone pinned upstream repos, build, install into upstream/install
#   scripts/server.sh run     run the installed server in the foreground (port 13327)
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

    # Python venv first, so configure finds its python3 and the plugin links against it.
    [ -d "$UP/venv" ] || python3 -m venv "$UP/venv"
    "$UP/venv/bin/pip" install -q -r "$UP/crossfire-maps/python/requirements.txt"

    BREW=$(brew --prefix)
    cd "$UP/crossfire-server"
    export PATH="$BREW/opt/flex/bin:$BREW/opt/libtool/libexec/gnubin:$PATH"
    export ACLOCAL_PATH="$BREW/share/aclocal"
    export PKG_CONFIG_PATH="$BREW/opt/sqlite/lib/pkgconfig:${PKG_CONFIG_PATH:-}"
    ./autogen.sh --prefix="$PREFIX"
    make -j"$(sysctl -n hw.ncpu)"
    make install
    ln -sfn "$UP/crossfire-maps" "$PREFIX/share/crossfire/maps"
}

run() {
    PYTHONHOME= PATH="$UP/venv/bin:$PATH" exec "$PREFIX/bin/crossfire-server" "$@"
}

case "${1:-}" in
    build) build ;;
    run) shift; run "$@" ;;
    *) echo "usage: $0 build|run" >&2; exit 1 ;;
esac
