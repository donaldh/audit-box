#!/bin/bash
set -e

# Create a unique temporary directory in /tmp
TMPDIR=$(mktemp -d /tmp/bwrap-overlay-XXXXXX)

# Create overlay and work directories
mkdir -p "$TMPDIR/overlay" "$TMPDIR/work"

# Cleanup function to remove tmpdir on exit
cleanup() {
    rm -rf "$TMPDIR"
}
trap cleanup EXIT

# Launch bubblewrap with overlay filesystem
exec bwrap \
    --ro-bind / / \
    --tmpfs /tmp \
    --unshare-pid \
    --overlay-src /home \
    --overlay "$TMPDIR/overlay" "$TMPDIR/work" /home \
    --dev /dev \
    --new-session \
    "$@"
