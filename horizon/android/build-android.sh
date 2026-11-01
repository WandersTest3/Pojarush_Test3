#!/usr/bin/env bash
# Builds Horizon's Android binaries for Pojarush_Test3 and copies them into the launcher's jniLibs.
#   ANDROID_NDK_HOME=/path/to/ndk/25.2.9519653 ./horizon/android/build-android.sh
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
HORIZON="$(cd "$HERE/.." && pwd)"
LAUNCHER="$(cd "$HORIZON/.." && pwd)"
JNI="$LAUNCHER/Pojarush/src/main/jniLibs/arm64-v8a"
: "${ANDROID_NDK_HOME:?set ANDROID_NDK_HOME to an Android NDK (r25 or newer)}"
API=26
TC="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin"

export CC_aarch64_linux_android="$TC/aarch64-linux-android$API-clang"
export CXX_aarch64_linux_android="$TC/aarch64-linux-android$API-clang++"
export AR_aarch64_linux_android="$TC/llvm-ar"
export RANLIB_aarch64_linux_android="$TC/llvm-ranlib"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$TC/aarch64-linux-android$API-clang"
TARGET_DIR="${CARGO_TARGET_DIR:-$HORIZON/target}"

rustup target add aarch64-linux-android
cd "$HORIZON"
cargo build --profile android --target aarch64-linux-android -p horizon-android -p horizon-prepare

mkdir -p "$JNI"
cp "$TARGET_DIR/aarch64-linux-android/android/libhorizon_android.so" "$JNI/libhorizon_android.so"
cp "$TARGET_DIR/aarch64-linux-android/android/horizon-prepare" "$JNI/libhorizon_prepare.so"
"$CC_aarch64_linux_android" -O2 -fPIE -pie -Wall -o "$JNI/libpojajava.so" "$HERE/pojajava.c" -ldl
"$TC/llvm-strip" "$JNI/libpojajava.so"
echo "Copied libhorizon_android.so, libhorizon_prepare.so and libpojajava.so to $JNI"
