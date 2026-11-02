#!/bin/sh
# Packs an iOS build of horizon-client into an unsigned Horizon.ipa (docs/ios.md). macOS only:
# it uses sips for the icons and plutil to check the Info.plist. SideStore/AltStore sign the
# .ipa with the player's Apple ID when they install it.
#
#   tools/ios_package.sh target/aarch64-apple-ios/release/horizon-client [out-dir]
set -eu
binary=$1
out=${2:-.local/ios}
root=$(cd "$(dirname "$0")/.." && pwd)
version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/crates/horizon-client/Cargo.toml" | head -n 1)
build=${GITHUB_RUN_NUMBER:-1}

rm -rf "$out/Payload" "$out/Horizon.ipa"
app="$out/Payload/Horizon.app"
mkdir -p "$app"
cp "$binary" "$app/Horizon"
chmod 755 "$app/Horizon"
sed -e "s/@VERSION@/$version/" -e "s/@BUILD@/$build/" \
    "$root/packaging/ios/Info.plist" > "$app/Info.plist"
plutil -lint "$app/Info.plist"
# The icon: 60 pt at 2x and 3x, from the 512 px PNG.
icon="$root/packaging/icons/hicolor/512x512/apps/horizon.png"
sips -z 120 120 "$icon" --out "$app/AppIcon60x60@2x.png" >/dev/null
sips -z 180 180 "$icon" --out "$app/AppIcon60x60@3x.png" >/dev/null
printf 'APPL????' > "$app/PkgInfo"
(cd "$out" && zip -qry Horizon.ipa Payload)
echo "$out/Horizon.ipa"
