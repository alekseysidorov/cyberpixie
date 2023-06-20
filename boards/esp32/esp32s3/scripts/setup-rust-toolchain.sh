#!/usr/bin/env bash

set -euf -o pipefail

# Install esp toolchain
ESP_TARGET_DIR="../target/esp"
EXPORT_FILE="$ESP_TARGET_DIR/export-esh.sh"
if [ ! -f "$EXPORT_FILE" ]; then
    mkdir -p "$ESP_TARGET_DIR"
    espup install -f $EXPORT_FILE -t esp32s3
else
    espup update
fi
. $EXPORT_FILE

export RUSTUP_TOOLCHAIN="esp"
