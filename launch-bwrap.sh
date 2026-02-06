#!/bin/bash
set -e

# Create a unique temporary directory in /tmp
TMPDIR=$(mktemp -d /tmp/bwrap-overlay-XXXXXX)

# Create overlay and work directories
mkdir -p "$TMPDIR/overlay" "$TMPDIR/work"

#create the seccomp policy
SECCOMP_FILE="$TMPDIR/filter.bpf"
python3 gen_seccomp.py "$SECCOMP_FILE"

# Cleanup function to remove tmpdir on exit
cleanup() {
    rm -rf "$TMPDIR"
}
trap cleanup EXIT

# Launch bubblewrap with overlay filesystem
exec bwrap \
    --ro-bind /usr /usr \
    --ro-bind /lib /lib \
    --ro-bind /bin /bin \
    --ro-bind /sbin /sbin \
    --ro-bind /etc /etc \
    --dir /tmp \
    --tmpfs /tmp \
    --ro-bind ~/.mitmproxy/mitmproxy-ca-cert.pem /tmp/mitmproxy-ca.pem \
    --unshare-pid \
    --overlay-src /home \
    --overlay "$TMPDIR/overlay" "$TMPDIR/work" /home \
    --dev-bind /dev /dev \
    --proc /proc \
    --die-with-parent \
    --seccomp 10 10< "$SECCOMP_FILE" \
    "$@"

