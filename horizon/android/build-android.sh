#!/usr/bin/env bash
# Builds Horizon's Android binaries for Pojarush_Test3 (arm64-v8a and armeabi-v7a) and copies them
# into the launcher's jniLibs.
#   ANDROID_NDK_HOME=/path/to/ndk/25.2.9519653 ./horizon/android/build-android.sh
#   ABIS="arm64-v8a" builds one of them only.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
HORIZON="$(cd "$HERE/.." && pwd)"
LAUNCHER="$(cd "$HORIZON/.." && pwd)"
: "${ANDROID_NDK_HOME:?set ANDROID_NDK_HOME to an Android NDK (r25 or newer)}"
API=26
TC="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin"
TARGET_DIR="${CARGO_TARGET_DIR:-$HORIZON/target}"

export CC_aarch64_linux_android="$TC/aarch64-linux-android$API-clang"
export CXX_aarch64_linux_android="$TC/aarch64-linux-android$API-clang++"
export AR_aarch64_linux_android="$TC/llvm-ar"
export RANLIB_aarch64_linux_android="$TC/llvm-ranlib"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$TC/aarch64-linux-android$API-clang"
export CC_armv7_linux_androideabi="$TC/armv7a-linux-androideabi$API-clang"
export CXX_armv7_linux_androideabi="$TC/armv7a-linux-androideabi$API-clang++"
export AR_armv7_linux_androideabi="$TC/llvm-ar"
export RANLIB_armv7_linux_androideabi="$TC/llvm-ranlib"
export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER="$TC/armv7a-linux-androideabi$API-clang"

cd "$HORIZON"
for ABI in ${ABIS:-arm64-v8a armeabi-v7a}; do
  case "$ABI" in
    arm64-v8a) TARGET=aarch64-linux-android; CC="$CC_aarch64_linux_android"; FEATURES=(--features horizon-android/mods) ;;
    armeabi-v7a) TARGET=armv7-linux-androideabi; CC="$CC_armv7_linux_androideabi"; FEATURES=() ;;
    *) echo "unknown ABI $ABI" >&2; exit 1 ;;
  esac
  rustup target add "$TARGET"
  cargo build --profile android --target "$TARGET" -p horizon-android "${FEATURES[@]}" -p horizon-prepare
  JNI="$LAUNCHER/Pojarush/src/main/jniLibs/$ABI"
  mkdir -p "$JNI"
  cp "$TARGET_DIR/$TARGET/android/libhorizon_android.so" "$JNI/libhorizon_android.so"
  cp "$TARGET_DIR/$TARGET/android/horizon-prepare" "$JNI/libhorizon_prepare.so"
  "$CC" -O2 -fPIE -pie -Wall -o "$JNI/libpojajava.so" "$HERE/pojajava.c" -ldl
  "$TC/llvm-strip" "$JNI/libpojajava.so"
  echo "Copied libhorizon_android.so, libhorizon_prepare.so and libpojajava.so to $JNI"
done
