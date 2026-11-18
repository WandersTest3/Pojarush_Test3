#!/bin/sh
set -e
jar="$1"
[ -f "$jar" ] || { echo "usage: $0 ACV-Cape-Fabric-<version>.jar" >&2; exit 1; }
dir="$(dirname "$0")/models/capes"
mkdir -p "$dir"
unzip -joq "$jar" 'assets/acvcape/textures/cape/*.png' -d "$dir"
