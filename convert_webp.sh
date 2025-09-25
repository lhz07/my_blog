#!/bin/bash
# Recursively convert all PNG and JPG files in a directory to WebP.
# If the WebP file already exists, it will be skipped.

# Usage: ./convert_to_webp.sh /path/to/folder

# Check for input folder
if [ -z "$1" ]; then
    echo "Usage: $0 /path/to/folder"
    exit 1
fi

INPUT_DIR="$1"

# Find all PNG and JPG files recursively
find "$INPUT_DIR" -type f \( -iname "*.png" -o -iname "*.jpg" -o -iname "*.jpeg" \) | while read -r img_file; do
    # Generate webp file path (same folder, same name)
    webp_file="${img_file%.*}.webp"

    # Skip if webp already exists
    if [ -f "$webp_file" ]; then
        echo "Skipping existing: $webp_file"
        continue
    fi

    # Convert PNG/JPG -> WebP
    echo "Converting: $img_file -> $webp_file"
    cwebp -q 80 "$img_file" -o "$webp_file" > /dev/null 2>&1

    # Check success
    if [ $? -ne 0 ]; then
        echo "Failed: $img_file"
    fi
done
