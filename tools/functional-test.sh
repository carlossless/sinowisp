#!/usr/bin/env bash

set -euo pipefail

# TOOL=sinowealth-kb-tool
TOOL="cargo run --"
FILE_PREFIX="private/test-$(date +'%Y%m%dT%H%M%S')"
PART="nuphy-air60"

EXPECTED_BOOTLOADER_MD5="3e0ebd0c440af5236d7ff8872343f85d"

FILE_DEFAULT="$FILE_PREFIX-read.hex"
FILE_BOOTLOADER="$FILE_PREFIX-read-bootloader.hex"
FILE_FULL="$FILE_PREFIX-read-full.hex"
FILE_CUSTOM="$FILE_PREFIX-read-custom.hex"
FILE_OVERRIDE="$FILE_PREFIX-read-override.hex"
FILE_POST_WRITE="$FILE_PREFIX-post-write.hex"
FILE_POST_WRITE_CUSTOM="$FILE_PREFIX-post-write-custom.hex"

function reboot_device () {
    echo "Cycling port power..."
    uhubctl -a cycle -l "3-3.3.4.4" -p 4 -d 1
    echo "Waiting..."
    sleep 5
}

function get_md5 () {
    MD5SUM=($(md5sum "$1"))
    echo $MD5SUM
}

function get_md5_from_hex () {
    objcopy --input-target=ihex --output-target=binary "$1" "${1%.hex}.bin"
    MD5SUM=$(get_md5 "${1%.hex}.bin")
    echo $MD5SUM
}

echo "Initial reboot..."
reboot_device

echo "Standard read..."
$TOOL read --part "$PART" "$FILE_DEFAULT"

reboot_device

echo "Bootloader read..."
$TOOL read --part "$PART" -b "$FILE_BOOTLOADER"

reboot_device

echo "Full read..."
$TOOL read --part "$PART" --full "$FILE_FULL"

reboot_device

echo "Custom read..."
$TOOL read \
    --firmware_size 61440 \
    --vendor_id 0x05ac \
    --product_id 0x024f \
    --bootloader_size 4096 \
    --page_size 2048 \
    --isp_iface_num 1 \
    --isp_usage_page 0xff00 \
    --isp_usage 0x0001 \
    --isp_index 1 \
    "$FILE_CUSTOM"

reboot_device

echo "Override read..."
$TOOL read \
    --part "$PART" \
    --vendor_id 0x05ac \
    --product_id 0x024f \
    "$FILE_OVERRIDE"

reboot_device

READ_MD5=$(get_md5_from_hex "$FILE_DEFAULT")
READ_BOOTLOADER_MD5=$(get_md5_from_hex "$FILE_BOOTLOADER")
READ_FULL_MD5=$(get_md5_from_hex "$FILE_FULL")
READ_CUSTOM_MD5=$(get_md5_from_hex "$FILE_CUSTOM")
READ_OVERRIDE_MD5=$(get_md5_from_hex "$FILE_OVERRIDE")

echo "Checking bootloader checksum"
if [[ "$READ_BOOTLOADER_MD5" != "$EXPECTED_BOOTLOADER_MD5" ]]; then
    echo "MD5 mismatch $READ_BOOTLOADER_MD5 != $EXPECTED_BOOTLOADER_MD5"
    exit 1
fi

echo "Checking custom checksum"
if [[ "$READ_CUSTOM_MD5" != "$READ_MD5" ]]; then
    echo "MD5 mismatch $READ_CUSTOM_MD5 != $READ_MD5"
    exit 1
fi

echo "Checking override checksum"
if [[ "$READ_OVERRIDE_MD5" != "$READ_MD5" ]]; then
    echo "MD5 mismatch $READ_OVERRIDE_MD5 != $READ_MD5"
    exit 1
fi

echo "Checking standard+bootloader == full"
cat "${FILE_DEFAULT%.hex}.bin" "${FILE_BOOTLOADER%.hex}.bin" > "$FILE_PREFIX-concat-full.bin"
EXPECTED_FULL_MD5=$(get_md5 "$FILE_PREFIX-concat-full.bin")
if [[ "$READ_FULL_MD5" != "$EXPECTED_FULL_MD5" ]]; then
    echo "MD5 mismatch $READ_FULL_MD5 != $EXPECTED_FULL_MD5"
    exit 1
fi

echo "Standard write..."
$TOOL write --part "$PART" "$FILE_DEFAULT"

reboot_device

echo "Post-write read..."
$TOOL read --part "$PART" "$FILE_POST_WRITE"

READ_POST_WRITE_MD5=$(get_md5_from_hex "$FILE_POST_WRITE")

echo "Checking post-write checksum"
if [[ "$READ_POST_WRITE_MD5" != "$READ_MD5" ]]; then
    echo "MD5 mismatch $READ_POST_WRITE_MD5 != $READ_MD5"
    exit 1
fi

reboot_device

echo "Custom write..."
$TOOL write \
    --firmware_size 61440 \
    --vendor_id 0x05ac \
    --product_id 0x024f \
    --bootloader_size 4096 \
    --page_size 2048 \
    --isp_iface_num 1 \
    --isp_usage_page 0xff00 \
    --isp_usage 0x0001 \
    --isp_index 1 \
    "$FILE_DEFAULT"

reboot_device

echo "Post-write read..."
$TOOL read --part "$PART" "$FILE_POST_WRITE_CUSTOM"

READ_POST_WRITE_CUSTOM_MD5=$(get_md5_from_hex "$FILE_POST_WRITE_CUSTOM")

echo "Checking post-write checksum"
if [[ "$READ_POST_WRITE_CUSTOM_MD5" != "$READ_MD5" ]]; then
    echo "MD5 mismatch $READ_POST_WRITE_CUSTOM_MD5 != $READ_MD5"
    exit 1
fi

reboot_device

echo "Passed all tests!"
