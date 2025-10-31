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

Usage
-----

```bash
audit-box --base /home --overlay /tmp/overlay
```

### Key Bindings

**Navigation:**
- `↑` / `↓` - Navigate file list (when file list pane is active) or scroll content (when content pane is active)
- `Tab` - Switch focus between file list pane and content pane

**File Selection:**
- `Space` - Toggle selection of current file/directory
  - For files: toggles selection on/off
  - For directories: toggles selection for all files within the directory
  - Deselecting a file automatically deselects all parent directories

**Actions:**
- `a` - Apply selected files (shows confirmation dialog)
  - Copies selected files from overlay to base filesystem
  - Verifies each copy by comparing file contents
  - Deletes files from overlay after successful verification
- `k` - Discard current file/directory (shows confirmation dialog)
  - Permanently deletes the file/directory from overlay filesystem
  - Cannot be undone

**Dialog Navigation:**
- `Left` / `Right` / `Tab` - Switch between OK/Discard and Cancel buttons
- `Enter` - Confirm selected action
- `Esc` - Close dialog without taking action

**General:**
- `q` - Quit the application

### Display Elements

**File Status Indicators:**
- `[N]` (green) - New file (does not exist in base filesystem)
- `[M]` (yellow) - Modified file (exists in base filesystem with different content)

**Selection Indicators:**
- `[ ]` - File is not selected
- `[✓]` - File is selected for application

**Content Pane:**
- For new files: displays file contents
- For modified files: displays unified diff with color-coded changes
  - Lines starting with `+` (green) - additions
  - Lines starting with `-` (red) - deletions
  - Lines starting with `---` / `+++` (cyan) - file headers
