#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TMP_DIR="$(mktemp -d /tmp/eevideo-test-build-config-XXXXXX)"
SYSROOT_DIR="$TMP_DIR/sysroot"
FAKE_BIN_DIR="$TMP_DIR/bin"
ARGS_FILE="$TMP_DIR/cargo-args.txt"
ENV_FILE="$TMP_DIR/cargo-env.txt"
trap 'rm -rf "$TMP_DIR"' EXIT

mkdir -p "$SYSROOT_DIR" "$FAKE_BIN_DIR"

cat > "$FAKE_BIN_DIR/cargo" <<EOF
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "\$*" > "$ARGS_FILE"
env | sort > "$ENV_FILE"
EOF
chmod +x "$FAKE_BIN_DIR/cargo"

PATH="$FAKE_BIN_DIR:$PATH" bash "$ROOT_DIR/cross/jetson-orin/build.sh" "$SYSROOT_DIR"

assert_contains() {
  local haystack="$1"
  local needle="$2"
  if [[ "$haystack" != *"$needle"* ]]; then
    echo "expected to find: $needle" >&2
    exit 1
  fi
}

captured_args="$(cat "$ARGS_FILE")"
captured_env="$(cat "$ENV_FILE")"

assert_contains "$captured_args" "build --release --target aarch64-unknown-linux-gnu -p gst-plugin-eevideo -p eedeviced"
assert_contains "$captured_env" "CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc"
assert_contains "$captured_env" "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc"
assert_contains "$captured_env" "PKG_CONFIG_SYSROOT_DIR=$SYSROOT_DIR"
assert_contains "$captured_env" "PKG_CONFIG_LIBDIR=$SYSROOT_DIR/usr/lib/aarch64-linux-gnu/pkgconfig:$SYSROOT_DIR/usr/lib/pkgconfig:$SYSROOT_DIR/usr/share/pkgconfig"
assert_contains "$captured_env" "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUSTFLAGS=-C link-arg=--sysroot=$SYSROOT_DIR"

echo "build wrapper configuration looks correct"
