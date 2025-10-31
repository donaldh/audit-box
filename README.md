audit-box
=========

Run a `command` in a sandbox to audit its behaviour and review filesystem changes.

Use [Bubblewrap](https://github.com/containers/bubblewrap) to create an on-the-fly sandbox with
a read-write overlay FS, with the host FS mounted read-only.

```bash
mkdir /tmp/overlay /tmp/work

bwrap \
    --ro-bind / / \
    --overlay-src /home --overlay /tmp/overlay /tmp/work /home \
    --tmpfs /tmp \
    --dev /dev \
    --unshare-pid \
    --new-session \
    ${command}
```

Use `audit-box` to review the FS changes made by `command` and selectively apply them to the
host FS.

```bash
audit-box --base /home --overlay /tmp/overlay
```
