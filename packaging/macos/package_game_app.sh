#!/usr/bin/env bash
# Packages the standalone macOS game client into Horizon.app, a .zip archive, and a drag-to-Applications .dmg.
# Supports Apple Silicon (arm64), Intel (x64), and Universal (universal, combining two Mach-O binaries via lipo).
#
# Usage:
#   packaging/macos/package_game_app.sh <BINARY_OR_ARM64_BIN> <VERSION> <OUTDIR> [arm64|x64|universal] [X64_BIN_FOR_UNIVERSAL]
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
bin_primary=$1
version=$2
out=$(mkdir -p "$3" && cd "$3" && pwd)
arch=${4:-arm64}
bin_secondary=${5:-}

stage_root=$(mktemp -d "${TMPDIR:-/tmp}/horizon-macos-pkg.XXXXXX")
trap 'rm -rf "$stage_root"' EXIT

app="$stage_root/Horizon.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"

case "$arch" in
  arm64)
    [[ "$(lipo -archs "$bin_primary")" == *arm64* ]] || { echo "$bin_primary is $(lipo -archs "$bin_primary"), expected arm64" >&2; exit 1; }
    cp "$bin_primary" "$app/Contents/MacOS/horizon-client"
    ;;
  x64)
    [[ "$(lipo -archs "$bin_primary")" == *x86_64* ]] || { echo "$bin_primary is $(lipo -archs "$bin_primary"), expected x86_64" >&2; exit 1; }
    cp "$bin_primary" "$app/Contents/MacOS/horizon-client"
    ;;
  universal)
    [[ -n "$bin_secondary" ]] || { echo "universal mode requires both arm64 and x64 binaries" >&2; exit 1; }
    lipo -create -output "$app/Contents/MacOS/horizon-client" "$bin_primary" "$bin_secondary"
    ;;
  *)
    echo "unknown architecture '$arch' (expected arm64, x64, or universal)" >&2
    exit 1
    ;;
esac

chmod 755 "$app/Contents/MacOS/horizon-client"
cp "$here/../icons/horizon.icns" "$app/Contents/Resources/horizon.icns"
sed "s/@VERSION@/$version/g" "$here/GameInfo.plist" > "$app/Contents/Info.plist"
printf 'APPL????' > "$app/Contents/PkgInfo"
plutil -lint "$app/Contents/Info.plist"

identity=${MACOS_SIGN_IDENTITY:--}
if [[ "$identity" == "-" ]]; then
  codesign --force --deep --sign - "$app"
else
  codesign --force --deep --options runtime --timestamp --sign "$identity" "$app"
fi
codesign --verify --strict --verbose=2 "$app"

# Verify only system libraries are linked.
others=$(otool -L "$app/Contents/MacOS/horizon-client" | tail -n +2 | awk '{print $1}' | grep -v -E '^(/usr/lib/|/System/Library/)' || true)
[[ -z "$others" ]] || { echo "unexpected non-system dylibs linked: $others" >&2; exit 1; }

# Smoke-check --help startup.
"$app/Contents/MacOS/horizon-client" --help | grep -q -- '--resources'

zip_out="$out/horizon-macos-$arch-app.zip"
dmg_out="$out/horizon-macos-$arch.dmg"
rm -f "$zip_out" "$dmg_out"

(cd "$stage_root" && ditto -c -k --norsrc --noextattr --noqtn --keepParent Horizon.app "$zip_out")

dmg_folder="$stage_root/dmg_root"
mkdir -p "$dmg_folder"
cp -R "$app" "$dmg_folder/Horizon.app"
ln -s /Applications "$dmg_folder/Applications"

hdiutil create -volname "Horizon ($arch)" -srcfolder "$dmg_folder" -ov -format UDZO "$dmg_out" >/dev/null
hdiutil verify "$dmg_out" >/dev/null

rm -rf "$out/Horizon-$arch.app"
cp -R "$app" "$out/Horizon-$arch.app"

echo "Packaged:"
echo "  App : $out/Horizon-$arch.app"
echo "  ZIP : $zip_out"
echo "  DMG : $dmg_out"
