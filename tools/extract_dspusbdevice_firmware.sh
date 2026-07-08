#!/bin/sh
set -eu

if [ "$#" -lt 1 ] || [ "$#" -gt 2 ]; then
    echo "usage: $0 /path/to/dspusbdevice.dll [output-directory]" >&2
    exit 2
fi

source_dll=$1
output_dir=${2:-artifacts/dspusbdevice_firmware}
expected_dll_sha256=567892d75c7243b3cc30487f7431933e593f3c288ea6add84c21d4e8678f877b

actual_dll_sha256=$(sha256sum "$source_dll" | cut -d ' ' -f 1)
if [ "$actual_dll_sha256" != "$expected_dll_sha256" ]; then
    echo "error: unsupported dspusbdevice.dll" >&2
    echo "expected SHA-256: $expected_dll_sha256" >&2
    echo "actual SHA-256:   $actual_dll_sha256" >&2
    exit 1
fi

mkdir -p "$output_dir"

# All four images are in .rdata. For this PE, raw_offset = RVA - 0xe00.
extract() {
    name=$1
    offset=$2
    size=$3
    expected_sha256=$4
    output=$output_dir/$name

    dd if="$source_dll" of="$output" bs=1 skip="$offset" count="$size" status=none
    actual_sha256=$(sha256sum "$output" | cut -d ' ' -f 1)
    if [ "$actual_sha256" != "$expected_sha256" ]; then
        echo "error: extracted hash mismatch for $name" >&2
        exit 1
    fi
    printf '%s  %s\n' "$actual_sha256" "$output"
}

extract revelator_io44_rev0122.bin 981280 1206028 80651beff220ec497f64cea95982c223f868e75e8a49f648d20f55359492c9ce
extract revelator_io24_rev0128.bin 2187312 1222292 790f668608448b7c9f244c360ad72e3c51f71b4e8a3d828eab0e08cd03b35de1
extract revelator_dynamic_rev0120.bin 3409616 486384 21e78a5e8ea4ce9c5c80ee95050729b0ebcb142bfc942848e91a75cb2eb11497
extract revelator_rev0157.bin 3896000 486408 c5234af41446bf295585b763f5ef1886682cc33303f81805de7ebc418bc00e05
