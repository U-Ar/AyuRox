#!/usr/bin/env bash
set -euo pipefail

# Usage: ./run_tests.sh <root_directory>
# Example: ./run_tests.sh test
EXT="lox"
ROOT_DIR="${1:-test}"

if [[ ! -d "${ROOT_DIR}" ]]; then
  echo "Directory not found: ${ROOT_DIR}" >&2
  exit 2
fi
found=0

echo "Searching ${ROOT_DIR}/**/*.${EXT}"
echo

find "${ROOT_DIR}" -type f -name "*.${EXT}" -print0 \
| LC_ALL=C sort -z \
| while IFS= read -r -d '' f; do
    found=1

    echo "============================================================"
    echo "FILE: ${f}"
    echo "CMD : cargo run -- \"${f}\""
    echo "------------------------------------------------------------"

    set +e
    cargo run -- "${f}"
    status=$?
    set -e

    echo "------------------------------------------------------------"
    echo "EXIT: ${status}"
    echo
done

if [[ "${found}" -eq 0 ]]; then
  echo "No files found."
fi