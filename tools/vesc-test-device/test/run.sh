#!/bin/sh
set -eu

test_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
output="${TMPDIR:-/tmp}/float-view-control-protocol-test"

cc -std=c11 -Wall -Wextra -Werror \
  "$test_dir/../main/control_protocol.c" \
  "$test_dir/control_protocol_test.c" \
  -o "$output"
"$output"
