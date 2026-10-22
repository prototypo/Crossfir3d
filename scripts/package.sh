#!/bin/bash
# Builds a download in dist/: Crossfir3d.dmg on a Mac, Crossfir3d-windows.zip on Windows
# (Git Bash). Unsigned. Needs the sounds in upstream/crossfire-sounds (or SOUNDS=dir).
# ponytail: the Mac app is for the build machine's chip only; lipo two targets for Intel too.
set -euo pipefail
cd "$(dirname "$0")/.."
SOUNDS="${SOUNDS:-upstream/crossfire-sounds}"
[ -f "$SOUNDS/sounds.conf" ] || { echo "no sounds in $SOUNDS" >&2; exit 1; }
VERSION=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
cargo build --release -p crossfir3d
rm -rf dist && mkdir dist

# Only the files git tracks: not the ignored experiments or the paid pack.
bundle() { # bundle DIR: the assets, sounds and licences into DIR
    mkdir -p "$1/sounds"
    git archive HEAD assets COPYING | tar -x -C "$1"
    (cd "$SOUNDS" && tar -c --exclude .git .) | tar -x -C "$1/sounds"
}

case "$(uname -s)" in
Darwin)
    APP=dist/stage/Crossfir3d.app
    mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
    cp target/release/crossfir3d "$APP/Contents/MacOS/"
    bundle "$APP/Contents/Resources"
    cat > "$APP/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>Crossfir3d</string>
<key>CFBundleIdentifier</key><string>org.crossfire.crossfir3d</string>
<key>CFBundleExecutable</key><string>crossfir3d</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>$VERSION</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
EOF
    ln -s /Applications dist/stage/Applications
    hdiutil create -volname Crossfir3d -srcfolder dist/stage -ov -format UDZO dist/Crossfir3d.dmg
    rm -rf dist/stage
    ;;
MINGW* | MSYS*)
    mkdir -p dist/Crossfir3d
    cp target/release/crossfir3d.exe dist/Crossfir3d/
    bundle dist/Crossfir3d
    (cd dist && 7z a -bd Crossfir3d-windows.zip Crossfir3d >/dev/null)
    rm -rf dist/Crossfir3d
    ;;
*) echo "no package for $(uname -s)" >&2; exit 1 ;;
esac
ls -lh dist
