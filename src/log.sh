#!/usr/bin/env bash

OUTPUT="output.log"
SCRIPT_NAME="$(basename "$0")"

# Clear or create the output file
>"$OUTPUT"

echo "[*] Starting recursive file logging..."
echo "[*] Writing to: $OUTPUT"
echo

# Recursively find all files, excluding this script and the output log
find . -type f ! -name "$SCRIPT_NAME" ! -name "$OUTPUT" | while read -r file; do
    echo "[+] Logging: $file"
    echo "<$file>" >>"$OUTPUT"
    cat "$file" >>"$OUTPUT"
    echo -e "\n" >>"$OUTPUT"
done

echo
echo "[✓] Done! All file contents have been written to $OUTPUT"
