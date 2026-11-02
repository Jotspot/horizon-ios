#!/bin/sh
# `cargo check` of the iOS build on Linux, where there is no iOS SDK (docs/ios.md). The C build
# scripts (ring, mimalloc) only need to produce archives that are never linked, so a stand-in
# compiler answers cc-rs's clang probe and writes empty arm64 objects. Rust errors and cfg
# mistakes show as usual; anything that needs the SDK is checked for real on CI (ios.yml).
#
#   tools/ios_check.sh [extra cargo args]
set -eu
command -v clang >/dev/null || { echo "ios_check: needs clang" >&2; exit 1; }
command -v llvm-ar >/dev/null || { echo "ios_check: needs llvm-ar" >&2; exit 1; }
rustup target add aarch64-apple-ios >/dev/null
shim=$(mktemp -d)
trap 'rm -rf "$shim"' EXIT
mkdir -p "$shim/sdk/usr/include"
cat > "$shim/cc" <<'CC'
#!/bin/sh
for a in "$@"; do [ "$a" = "-E" ] && exec clang --target=arm64-apple-ios "$@"; done
out=""; prev=""
for a in "$@"; do [ "$prev" = "-o" ] && out="$a"; prev="$a"; done
[ -z "$out" ] && exit 1
echo "" | clang --target=arm64-apple-ios -x c -c - -o "$out"
CC
chmod +x "$shim/cc"
SDKROOT="$shim/sdk" CC_aarch64_apple_ios="$shim/cc" CXX_aarch64_apple_ios="$shim/cc" \
AR_aarch64_apple_ios=llvm-ar CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target}" \
cargo check --target aarch64-apple-ios -p horizon-client --bin horizon-client \
    --no-default-features --features audio "$@"
