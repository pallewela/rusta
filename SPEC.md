# `rusta` — Feature Specification

`rusta` is a macOS-only CLI for creating and managing Ubuntu VMs on Apple
Silicon using [Tart](https://tart.run/). It is the spiritual successor to
`ubuntu-tart-vm.sh`, but exposes its features through a subcommand-based UX
rather than a single mega-script.

This document specifies behavior; it does not prescribe implementation details
beyond what is required for parity with the existing script.

---

## 1. Runtime requirements

| Requirement       | Detail                                                                 |
| ----------------- | ---------------------------------------------------------------------- |
| Host architecture | `arm64` (Apple Silicon). Any other `uname -m` aborts at startup.       |
| Host OS           | macOS (relies on Tart + Apple Virtualization.framework).               |
| Required CLIs     | `brew` (must be present). `tart` and `sshpass` are auto-installed.     |
| Optional CLIs     | `docker` (host-side, installed automatically for `docker-setup`).      |
| Network           | Outbound HTTPS to `ghcr.io` (OCI image pulls and `rusta versions`).    |

Auto-installed via Homebrew on demand:

- `tart` from tap `cirruslabs/cli/tart`
- `sshpass` (whenever SSH-based steps are needed)
- `docker` (only for `rusta docker-setup`)

---

## 2. Command surface

```
rusta <command> [args...] [flags...]
```

### 2.1 Global flags

Accepted by every subcommand:

| Flag             | Default | Description                                            |
| ---------------- | ------- | ------------------------------------------------------ |
| `--verbose`      | off     | Verbose logging (equivalent to `set -x` + `LogLevel=INFO` on SSH). |
| `--log <file>`   | —       | Tee all stdout/stderr to the given file.               |
| `--help`, `-h`   | —       | Print help for the current (sub)command and exit 0.    |

`rusta` (no args) and `rusta --help` print top-level help and exit 0.

### 2.2 Subcommand summary

| Subcommand                       | Purpose                                                       |
| -------------------------------- | ------------------------------------------------------------- |
| `rusta up [<vm>]`                | Start a VM (headless by default).                             |
| `rusta down [<vm>]`              | Gracefully shut down a VM (`--force` to hard-stop).           |
| `rusta create [<vm>]`            | Create + provision a new Ubuntu VM.                           |
| `rusta delete <vm>`              | Delete a VM (Tart state). Requires confirmation or `--yes`.   |
| `rusta list`                     | List Tart VMs and indicate the current default.               |
| `rusta versions`                 | List available OCI tags across configured sources × images.   |
| `rusta source [add\|rm\|move]`   | Manage the image sources images are cloned from (see §4.12).   |
| `rusta image [add\|rm\|move]`    | Manage the image names (repos) cloned under each source (§4.13).|
| `rusta default [<vm>]`           | Print or set the default VM.                                  |
| `rusta ip [<vm>]`                | Print the guest IP of the VM.                                 |
| `rusta ssh [<vm>] [-- cmd...]`   | Open an SSH session (or run a command) on the VM.             |
| `rusta docker-setup [<vm>]`      | Install Docker in the VM and wire host SSH/Docker context.    |
| `rusta ssh-copy [<vm>]`          | Copy host `~/.ssh/id_*` and `*.pem` into the VM.              |

All subcommands that take `<vm>` accept it as a positional. If omitted, the
**default VM** is used (see §3).

### 2.3 Exit codes

| Code | Meaning                                                                       |
| ---- | ----------------------------------------------------------------------------- |
| 0    | Success (including `--help`, `versions`, no-op when target already in desired state). |
| 1    | Bad usage, unmet prerequisite, validation failure, or runtime error.          |
| 2    | VM not found (when an explicit name was given or default resolution failed).  |

---

## 3. The "default VM" concept

`rusta` maintains a single host-side **default VM** name so that `up`,
`down`, etc. can be invoked without arguments.

### 3.1 State file

- Location: `~/.local/share/rusta/state.toml` (parent dir auto-created).
- Schema:
  ```toml
  default_vm = "ubuntu-2404"

  [vms.ubuntu-2404]
  gui = false

  [[sources]]
  registry = "ghcr.io/cirruslabs"

  [[sources]]
  registry = "ghcr.io/pallewela"

  images = ["ubuntu", "ubuntu-desktop"]
  ```
  `vms.<name>.gui` records the `--gui` choice from `rusta create`. Used
  by `rusta up` to pick the default boot mode (§4.1). VMs created before
  this feature have no `[vms.<name>]` entry and default to headless boot.
  `[[sources]]` is the ordered list of image sources (§4.12); an absent or
  empty list means the seeded `ghcr.io/cirruslabs` default.
  `images` is the ordered list of image names (§4.13); an absent or empty
  list means the seeded `["ubuntu", "ubuntu-desktop"]` defaults. The first
  image (`ubuntu`) is the `create` default.

### 3.2 Resolution rule

When a subcommand needs a VM name and none is given on the command line:

1. If `default_vm` is set in `state.toml` **and** that VM exists in
   `tart list`, use it. Done.
2. If `default_vm` is unset (or names a VM that no longer exists), enumerate
   `tart list`:
   - **Zero VMs** → exit 2 with a hint to `rusta create`.
   - **One or more VMs** → **interactively prompt** the user to choose one.
     The chosen VM is persisted as `default_vm` before the command proceeds.
3. There is **no hardcoded fallback** (no implicit `ubuntu-2404`).

If stdin is not a TTY (non-interactive context, e.g. CI), the prompt cannot
run; instead `rusta` exits 2 with a message instructing the caller to pass
the VM explicitly or run `rusta default <vm>` first.

### 3.3 The interactive picker

Triggered by §3.2 step 2 when more than one VM exists, or as a confirmation
when exactly one exists:

```
No default VM is set. Pick one:
  1) ubuntu-2404   (stopped)
  2) lab-22        (running)
> 1
Set 'ubuntu-2404' as default for future commands.
```

- Lists all Tart VMs with their current status; selection by number.
- An empty answer or Ctrl-C aborts the command with exit 1 and does **not**
  write state.
- The chosen VM is written to `state.toml` immediately, before the original
  subcommand's work begins.

### 3.4 How the default gets set

The default is set only by these explicit paths — never as a side effect of
`create`, `up`, `down`, etc. when the VM is named on the command line:

- **The interactive picker** (§3.3), the first time the user runs a command
  that needs a default while none is set.
- **`rusta default <vm>`** — explicit set. Exits 2 if `<vm>` does not exist.

Notes:

- `rusta create <vm>` and `rusta create` (which interactively prompts for
  a name — see §4.3) both **leave the default untouched**. The next
  argument-less command that needs an existing VM triggers the picker.
- `rusta default` with no argument prints the currently-set default, or
  prints "no default set" and exits 1 if none is set. It never prompts.
- `rusta delete <vm>` clears the default if it pointed at the deleted VM.

---

## 4. Subcommand details

### 4.1 `rusta up [<vm>] [--graphical|-G|--graphics|--gui] [--no-gui|--no-graphics]`

Boot a VM.

- Resolves `<vm>` per §3.
- If the VM is already running, prints `[skip]` and exits 0.
- Boot mode follows the VM's `create`-time choice: VMs created with
  `--gui` boot with a graphics window; all others boot headless
  (`tart run <vm> --no-graphics`). VMs created before this feature have
  no recorded preference and default to headless.
- `--graphical` (aliases: `-G`, `--graphics`, `--gui`): force a graphics
  window, regardless of the recorded preference.
- `--no-gui` (alias: `--no-graphics`): force headless boot, even for
  GUI-enabled VMs.
- `--graphical` and `--no-gui` are mutually exclusive.
- Backgrounded with the PID written to `~/.local/share/rusta/run/<vm>.pid`
  so subsequent commands can find it.
- Waits for the **tart guest agent** (`tart exec <vm> true`, poll 2s × 60).
- Prints the guest IP once available (best-effort; not fatal if delayed).
- Does **not** re-run provisioning; that only happens during `create`.

### 4.2 `rusta down [<vm>] [--force] [--timeout <secs>]`

Stop a VM.

- Resolves `<vm>` per §3.
- If the VM is already stopped, prints `[skip]` and exits 0.
- **Graceful (default):** issues `sudo shutdown -h now` via `tart exec`,
  then waits up to `--timeout` seconds (default **60s**) for the `tart run`
  process to exit. If the timeout expires without a clean stop, exit 1 with
  a hint to retry with `--force`.
- **`--force` (alias `-f`):** skip the graceful path; call `tart stop <vm>`
  (or kill the recorded PID and fall back to `tart stop` if needed). Exit 1
  only if the VM is still running after the operation.
- Removes the stale `~/.local/share/rusta/run/<vm>.pid` on success.

### 4.3 `rusta create [<vm>] [flags]`

Clone + provision a new Ubuntu VM.

Flags:

| Flag                    | Default          | Description                                                                   |
| ----------------------- | ---------------- | ----------------------------------------------------------------------------- |
| `--version <ver>`       | `24.04`          | Ubuntu release line (OCI tag); resolved against configured sources (§4.12).   |
| `--image <name>`        | first image      | Image family (repo) to clone, e.g. `ubuntu-desktop`; defaults to the first configured image (§4.13). Composes with `--source`. |
| `--gui [pkg]`           | off / `ubuntu-desktop` | Install a desktop. Allowed: `ubuntu-desktop`, `xubuntu-desktop`, `lubuntu-desktop`, `lightdm`. |
| `--cpus <n>`            | `6`              | CPU count.                                                                    |
| `--memory <mb>`         | `8192`           | Memory in MB.                                                                 |
| `--disk <gb>`           | `80`             | Disk size in GB.                                                              |
| `--user <username>`     | `admin`          | Guest login username (image-dependent).                                       |
| `--password <password>` | `admin`          | Guest login password used by `sshpass`.                                       |
| `--ssh-copy-keys`       | off              | After provisioning, copy host SSH keys into the guest (see §4.10).            |
| `--debug-no-headless`   | off              | Run with a graphics window during provisioning (debug only).                  |
| `--source <registry>`   | (all sources)    | Pin resolution to one configured source, by registry prefix or label (§4.12). |
| `--image-ref <ref>`     | (unset)          | Clone this exact image reference verbatim, bypassing source + image resolution. Conflicts with `--source` and `--image`. |

Positional `<vm>` is the VM name. **`rusta create` never assumes a name**:
the default-VM mechanism (§3) does not apply, since `create` is producing a
new VM, not selecting an existing one. If `<vm>` is omitted:

- If stdin is a TTY, **interactively prompt** for the name, offering
  `<image>-<UBUNTU_VERSION_NODOT>` (e.g. default image + `--version 22.04`
  → `ubuntu-2204`; `--image ubuntu-desktop` → `ubuntu-desktop-2204`)
  as a suggested default the user can accept with an empty line:
  ```
  VM name [ubuntu-2404]:
  ```
  Ctrl-C or EOF aborts with exit 1 and creates nothing.
- If stdin is **not** a TTY, exit 1 with a message instructing the caller
  to pass the VM name on the command line. `create` never proceeds with a
  silently-synthesized name.

Name must match `^[a-zA-Z0-9][a-zA-Z0-9._-]*$`.

Behavior:

1. Validate platform/prereqs (arm64, brew, tart auto-install).
2. Resolve the VM name per the rule above (explicit arg or interactive
   prompt). The chosen name is **not** written to `state.default_vm`.
3. If the VM name already exists, **skip creation** and print a recreate
   hint (`rusta delete <vm> && rusta create <vm> ...`); no re-provisioning.
4. Otherwise:
   - Determine the **image** (§4.13): `--image <name>` if given, else the first
     configured image (`ubuntu` by default).
   - Resolve the image reference to clone (§4.12): `--image-ref` verbatim, else
     the first configured source (in priority order) that advertises
     `<image>:<version>`.
   - `tart clone <resolved-ref> <vm>` (e.g. `ghcr.io/cirruslabs/ubuntu:<version>`).
   - `tart set <vm> --cpu <n> --memory <mb> --disk-size <gb>`.
   - Generate `~/.local/share/rusta/provision/<vm>.sh` (kept for debugging).
   - Boot headlessly (or with window under `--debug-no-headless`).
   - Wait for guest agent; upload + execute provisioning script via
     `tart exec`; shut down cleanly. See §5 for the provisioning behavior.
5. **Does not** modify `state.default_vm` (see §3.4) — even when the name
   came from the interactive prompt.
6. If `--ssh-copy-keys`, run the `ssh-copy` flow against the new VM (§4.10),
   which transiently boots it again.

### 4.4 `rusta delete <vm> [--yes]`

Remove a VM from Tart's storage.

- Requires explicit `<vm>` (no default-VM fallback — too destructive to
  silently delete the default).
- Refuses to run if the VM is currently running (suggests `rusta down`
  first); `--force-running` to stop+delete in one shot.
- Prompts for confirmation unless `--yes` (`-y`) is given.
- Clears `state.default_vm` if it pointed at this VM.

### 4.5 `rusta list`

Print a table of all Tart VMs:

```
NAME          STATUS    DEFAULT
ubuntu-2404   running   *
lab-22        stopped
```

The `DEFAULT` column shows `*` next to the resolved default. Exits 0 even
if there are no VMs.

### 4.6 `rusta versions [--source <registry>] [--image <name>]`

List the available OCI tags across the configured **sources × images** matrix
(§4.12, §4.13). For each `(source, image)` cell's `<registry>/<image>` repository:

1. Fetch an anonymous pull token from `<host>/token`.
2. List tags from `<host>/v2/<namespace>/<image>/tags/list`.
3. Filter to tags matching `^\d+\.\d+$`.

Then merge, sort ascending, and print one per line, highlighting `24.04` as
`(default)`.

- With a **single** source **and** a single image, output is unannotated (one
  tag per line) — the legacy format.
- With multiple sources but a single image, each tag is annotated with the
  providing source label(s); when more than one source offers a tag, the one
  `create` would pick (first in priority order) is noted (`(create uses <label>)`).
- With **multiple images**, each tag line groups providers by image (in image
  priority order): `<image>: <src>, <src>  (create uses <src>)`. An image with
  no provider for a tag is omitted from that line.
- `--source <registry>` limits output to a single configured source (by registry
  prefix or label); `--image <name>` limits output to a single image. They compose.
- A `(source, image)` cell where the source **host is unreachable** is skipped
  with a warning. A cell where the source simply **lacks that image** (e.g. a 404)
  is a silent empty cell, not a failure. `versions` exits 1 only when **no** cell
  produced any result.

### 4.7 `rusta default [<vm>]`

- No arg: print the resolved default VM, or "no default set" + exit 1.
- With arg: set `state.default_vm = <vm>` (exit 2 if `<vm>` does not exist).

### 4.8 `rusta ip [<vm>]`

Print `tart ip <vm>` (waits up to 60s). Exit 1 if no IP is obtained.

### 4.9 `rusta ssh [<vm>] [-- cmd args...]`

- Resolves `<vm>` per §3.
- If the VM is not running, exits 1 (does **not** auto-`up`; suggest
  `rusta up <vm>`). Alternative: `--auto-up` flag to boot first.
- Connects via `sshpass -p <password> ssh <user>@<ip>` using the SSH options
  from §6.2.
- Anything after `--` is executed as a remote command; otherwise an
  interactive shell.

### 4.10 `rusta ssh-copy [<vm>]`

Copy host `~/.ssh/id_*` and `*.pem` files into the guest's `~/.ssh/`.

- Resolves `<vm>` per §3.
- Boots the VM if not running; shuts it back down at the end (only when
  `rusta` started it itself — same "started_by_us" pattern as today).
- Verifies host has `~/.ssh`; otherwise exit 1.
- Collects regular files matching `~/.ssh/id_*` and `~/.ssh/*.pem`. If
  none, prints `[skip]` and exits 0.
- Inside guest: `mkdir -p ~/.ssh && chmod 700 ~/.ssh`; `scp` the files;
  normalize permissions (`*.pub` → 644, others → 600; `chmod 700 ~/.ssh`).

### 4.11 `rusta docker-setup [<vm>]`

Install Docker Engine inside an existing VM and wire host-side
`docker context` + `~/.ssh/config` alias.

- Resolves `<vm>` per §3.
- Ensures host has `sshpass` and `docker` CLI (auto-install via Homebrew).
- Boots the VM if not running; shuts it back down at the end if started by
  `rusta`.
- Generates `~/.ssh/id_ed25519` (empty passphrase) if missing.
- `ssh-copy-id` the public key into the guest (password auth).
- Inside the guest, installs Docker via `curl -fsSL https://get.docker.com | sudo sh`
  **only if** `docker` is absent. Adds `$USER` to the `docker` group if not
  already a member. `systemctl enable --now docker`.
- On host: idempotently appends a `Host docker-<vm>` block to
  `~/.ssh/config` (pinned to the observed IP, `IdentitiesOnly yes`, strict
  host-key checking disabled). `chmod 600 ~/.ssh/config`.
- Idempotently creates a Docker context `docker-<vm>` pointing at
  `ssh://<user>@docker-<vm>`.
- Prints a summary including the SSH alias, the context name, the
  three-step usage hint, and the IP-pinning caveat.

### 4.12 `rusta source [list | add <registry> | rm <registry> | move <registry> <pos>]`

Manage the **image sources** that `create` and `versions` consider. A source is a
registry host + namespace prefix (e.g. `ghcr.io/cirruslabs`); rusta appends the
selected image name (§4.13, `ubuntu` by default) to form the repository. Sources
are an **ordered list** stored in the state file (§3.1) under `[[sources]]`; list
position is **priority**.

Model and resolution rules:

- **Seeded default.** When no sources are configured, rusta behaves as if a single
  source `ghcr.io/cirruslabs` were present, preserving prior single-source
  behavior. The default is materialized into the state file on the first mutation.
- **Conflict rule.** When a requested version exists in more than one source, the
  first source in priority (config) order wins.
- **`create` resolution.** With a single candidate source, the reference is built
  directly with no registry query. With two or more, each source's tags are
  queried (token + tag-list, as in §4.6) and the first advertising the version
  wins; unreachable sources are skipped with a warning. If no reachable source
  offers the version, `create` errors (exit 1) and creates nothing.
- **ghcr.io only (v1).** `add` accepts only `ghcr.io` hosts; other registries are
  rejected. `tart clone` itself works with any registry via `--image-ref`, but tag
  listing/aggregation is ghcr-specific for now.

Subactions:

- `list` (default when no subaction) — print configured sources in priority order;
  notes when the built-in default is in effect.
- `add <registry>` — validate + normalize (`<host>/<namespace>`, optional trailing
  `/ubuntu` stripped, no tag) and append. Duplicates are a no-op. First add
  materializes the seeded `cirruslabs` default ahead of the new entry.
- `rm <registry>` — remove by registry prefix; exit 2 if absent. Removing the last
  remaining source re-seeds the default (rusta is never sourceless).
- `move <registry> <pos>` — move a source to 1-based priority position `<pos>`
  (clamped); exit 2 if absent.

Sources are host-independent of Tart/Apple Silicon — `rusta source` skips the
arm64/brew/tart preflight.

### 4.13 `rusta image [list | add <name> | rm <name> | move <name> <pos>]`

Manage the **image names** (repositories) that rusta clones under each source. An
image is a single OCI repository segment (e.g. `ubuntu`, `ubuntu-desktop`); the
namespace comes from the source (§4.12). Images are a **global, ordered list**
stored in the state file (§3.1) under `images`; list position is **priority** and
the **first image is the `create` default**.

Model and resolution rules:

- **Seeded defaults.** When no images are configured, rusta behaves as if the
  images `ubuntu` and `ubuntu-desktop` were present (in that order), with `ubuntu`
  as the `create` default. The defaults are materialized into the state file on the
  first mutation.
- **Global, not per-source.** The list applies to every source. At `create` time
  the selected image is searched across sources (so `create` makes the same number
  of registry queries as before — the image is fixed before resolution). A source
  that does not host the selected image is treated like a source that lacks the
  version (skipped). `versions` walks the full source × image matrix.
- **Selection.** `rusta create` clones the first configured image unless `--image
  <name>` overrides it. `--image` need not be in the configured list (any valid
  name is accepted); `--image-ref` bypasses image resolution entirely.

Subactions:

- `list` (default when no subaction) — print configured images in priority order,
  marking the first as `(default)`; notes when the built-in default is in effect.
- `add <name>` — validate (single segment: no `/`, no `:` tag, lowercase OCI
  grammar) and append. Duplicates are a no-op. First add materializes the seeded
  defaults ahead of the new entry.
- `rm <name>` — remove by name; exit 2 if absent. Removing the last remaining image
  re-seeds the defaults (rusta is never imageless).
- `move <name> <pos>` — move an image to 1-based priority position `<pos>`
  (clamped); exit 2 if absent.

Like `rusta source`, `rusta image` skips the arm64/brew/tart preflight.

---

## 5. Provisioning (used by `rusta create`)

Behavior of the per-VM provisioning script is unchanged from the existing
implementation:

- Persists output to `/var/log/provision.log` inside the guest.
- Sets `DEBIAN_FRONTEND=noninteractive`, `DEBCONF_NONINTERACTIVE_SEEN=true`,
  `NEEDRESTART_MODE=l`, `LC_ALL=C.UTF-8`, `LANG=C.UTF-8`.
- Stops `unattended-upgrades` and `apt-daily{,-upgrade}.{service,timer}`
  and waits for the dpkg/apt lock (cap ~10 minutes).
- **Per-release apt cache fix:** for releases known to ship with stale ARM64
  apt cache files under `<codename>-updates` / `<codename>-security`, remove
  those files before `apt-get update` to avoid dependency-resolution failures.
  Currently applied to:
  - `24.04` (codename `noble`) — paths
    `/var/lib/apt/lists/ports.ubuntu.com_ubuntu-ports_dists_noble-{updates,security}_main_binary-arm64_Packages`.
  - `26.04` (codename per release) — same pattern, codename substituted.

  The mapping is data-driven: adding another affected release means adding
  a `{version, codename}` entry, not new code.
- Installs `apt-fast` (via PPA `ppa:apt-fast/stable`) for parallel apt.
- Always installs: `spice-vdagent`, `spice-webdavd`, `curl`, `wget`, `git`.
- Starts `spice-vdagent.socket` and `spice-vdagent.service` (best-effort).

When `--gui` is set:

- Before installing the desktop, pre-creates
  `/etc/NetworkManager/conf.d/10-manage-all.conf` with
  `unmanaged-devices=none`, so NetworkManager takes over from
  systemd-networkd cleanly.
- Installs the desktop meta-package and matching display manager:

  | `--gui` value     | Display manager |
  | ----------------- | --------------- |
  | `ubuntu-desktop`  | `gdm3`          |
  | `xubuntu-desktop` | `lightdm`       |
  | `lubuntu-desktop` | `sddm`          |
  | `lightdm`         | `lightdm`       |

- Restarts NetworkManager, disables
  `systemd-networkd-wait-online.service`, sets default target to
  `graphical.target`, enables the display manager, and suppresses the
  GNOME initial-setup wizard via `~/.config/gnome-initial-setup-done`.

---

## 6. Cross-cutting behavior

### 6.1 Polling timeouts

| Wait                    | Cadence | Cap        |
| ----------------------- | ------- | ---------- |
| Tart IP discovery       | 2s × 60 | ~2 min     |
| SSH readiness           | 3s × 40 | ~2 min     |
| Tart guest agent ready  | 2s × 60 | ~2 min     |
| Guest dpkg/apt lock     | 5s × 120| ~10 min    |
| `rusta down` grace      | 1s × `--timeout` (default 60) | configurable |

All timeouts are fatal on expiry (except graceful `down`, which suggests
`--force`).

### 6.2 SSH options

Used everywhere `rusta` shells into the guest:

```
StrictHostKeyChecking=no
UserKnownHostsFile=/dev/null
PubkeyAuthentication=no     # password auth is the default; ssh-copy-id flips this on a per-VM basis
LogLevel=ERROR              # INFO under --verbose
ConnectTimeout=10
ServerAliveInterval=30
ServerAliveCountMax=120
```

### 6.3 Process tracking

Background `tart run` processes started by `rusta` write their PID to
`~/.local/share/rusta/run/<vm>.pid`. `rusta down`, `delete`, and the
auto-shutdown tails in `ssh-copy` / `docker-setup` consult this file to
reap the right process. A signal trap kills + reaps the process on
`EXIT|INT|TERM` while `rusta` is the owner.

### 6.4 Logging and output conventions

- TTY-aware ANSI coloring (bold/green/yellow/red/cyan); collapses to empty
  strings when stdout is not a TTY.
- Prefixes: `==>` (info, cyan/bold), `[ok]` (green), `[skip]` (yellow),
  `[error]` (red, to stderr).
- `--log <file>` tees the entire run (including provisioning) to the file.

---

## 7. Filesystem and host-side artifacts

| Path                                            | Purpose                                                       |
| ----------------------------------------------- | ------------------------------------------------------------- |
| `~/.tart/vms/`                                  | VM storage (managed by Tart).                                 |
| `~/.tart/cache/`                                | OCI image cache (managed by Tart).                            |
| `~/.local/share/rusta/state.toml`               | Persistent `rusta` state (default VM, etc.).                  |
| `~/.local/share/rusta/provision/<vm>.sh`        | Generated provisioning script (kept after run for debugging). |
| `~/.local/share/rusta/run/<vm>.pid`             | Tracked PID of a `rusta`-launched `tart run`.                 |
| `~/.ssh/id_ed25519` / `.pub`                    | Auto-generated by `docker-setup` if absent.                   |
| `~/.ssh/config`                                 | Appended with `Host docker-<vm>` block by `docker-setup`.     |
| `<--log file>`                                  | Tee of stdout+stderr when `--log` is given.                   |

Inside the guest:

| Path                                                              | Purpose                            |
| ----------------------------------------------------------------- | ---------------------------------- |
| `/tmp/provision.sh`                                               | Uploaded provisioning script.      |
| `/var/log/provision.log`                                          | Full provisioning output log.      |
| `/etc/NetworkManager/conf.d/10-manage-all.conf` (gui only)        | Forces NM to manage all devices.   |
| `~/.config/gnome-initial-setup-done` (gui only)                   | Suppresses GNOME welcome wizard.   |

---

## 8. Idempotency

- `rusta up` on a running VM → `[skip]`.
- `rusta down` on a stopped VM → `[skip]`.
- `rusta create` with an existing name → `[skip]` + recreate hint; no
  re-provisioning, no resource change.
- `rusta default <vm>` is a pure state write.
- `rusta docker-setup` re-runs are safe: SSH key creation, `~/.ssh/config`
  block, and `docker context` are each guarded by existence checks.
- `rusta ssh-copy` re-runs overwrite the copied files but leave permissions
  correct.

---

## 9. Non-goals

- Non-Ubuntu-family guests; non-OCI Tart images; non-`ghcr.io` sources.
- Per-source image lists and per-image attributes (provisioning, default
  resources): images are a single global list in v1.
- Architectures other than `arm64`.
- Post-creation VM resize (CPU/memory/disk are set once at `create` time).
- Snapshot, suspend/resume, export, or registry-push workflows.
- Multi-VM batch operations.
- Windows or x86_64 Linux hosts.

---

## 10. Behavioral checklist

A working `rusta` should pass each of these end-to-end:

1. `rusta` (no args) → top-level help, exit 0.
2. `rusta --help` and `rusta <cmd> --help` → command-specific help, exit 0.
3. `rusta versions` → lists tags from ghcr.io, `24.04` flagged `(default)`.
4. `rusta create` (interactive, TTY stdin) → prompts `VM name [ubuntu-2404]:`;
   accepting the suggestion creates `ubuntu-2404` with 6 CPU / 8 GB / 80 GB,
   boots, provisions SPICE tools, shuts down. Non-TTY stdin → exits 1
   without creating anything, instructing the caller to pass the VM name.
   `state.default_vm` is **unchanged** in both branches.
5. `rusta create --version 22.04 lab` → creates `lab` from `:22.04`.
   `state.default_vm` is **unchanged**, even when a different default is set.
6. `rusta create --gui` / `--gui xubuntu-desktop` → installs the matching
   desktop and display manager with the NetworkManager workaround. Works
   for **every** Ubuntu version exposed by `rusta versions`, including
   24.04 and 26.04 (both apply the per-release apt cache fix from §5).
7. `rusta create lab --ssh-copy-keys` → after provisioning, transiently
   boots `lab`, copies host `id_*`/`*.pem` files, shuts it down.
8. `rusta up` (no arg, no default set, ≥1 VM exists) → interactive picker
   appears; chosen VM is written to `state.toml` then booted. Re-running
   `rusta up` with no arg now goes straight to that VM.
9. `rusta up` (no arg, no default set, 0 VMs exist) → exit 2 with a hint to
   `rusta create`.
10. `rusta up` (no arg, no default set, non-TTY stdin) → exit 2 without
    prompting; message instructs the caller to pass a VM or run `rusta default`.
11. `rusta up` (no arg, default is set and exists) → boots the default
    headlessly; second invocation is a `[skip]`.
12. `rusta up lab` (explicit name) → boots `lab` regardless of default;
    `state.default_vm` is **unchanged**.
13. Boot-mode defaults track the `create`-time `--gui` choice:
    - `rusta create lab` then `rusta up lab` → boots headless.
    - `rusta create lab --gui` then `rusta up lab` → boots with a
      graphics window with no explicit flag.
    - `rusta up lab --no-gui` (or `--no-graphics`) on a GUI-enabled VM →
      boots headless for that invocation.
    - `rusta up lab --graphical` (or `-G` / `--graphics` / `--gui`) on a
      headless VM → boots with a graphics window for that invocation.
    - `rusta up lab --graphical --no-gui` → exits 1 (mutually exclusive).
14. `rusta down` → graceful shutdown of the default VM within 60s; second
    invocation is `[skip]`. Picker triggers if no default is set.
15. `rusta down lab --force` → hard-stops `lab` even if guest agent is
    unresponsive.
16. `rusta down --timeout 5` → if the guest does not stop within 5s, exits
    1 with a "retry with --force" hint.
17. `rusta list` → tabular VM listing with `*` next to the default (if any).
18. `rusta default` (no arg, none set) → prints "no default set" + exit 1
    (no prompt).
19. `rusta default lab` → sets default to `lab`; exits 2 if `lab` is unknown.
20. `rusta delete lab` → prompts; with `--yes` deletes without prompt;
    clears default if it pointed at `lab`.
21. `rusta ip` / `rusta ip lab` → prints the guest IP.
22. `rusta ssh lab` → interactive SSH session (after `rusta up lab`).
23. `rusta ssh lab -- uname -a` → runs the remote command and exits.
24. `rusta ssh-copy` / `rusta ssh-copy lab` → copies host SSH keys with
    correct permissions, idempotent on re-run.
25. `rusta docker-setup` / `rusta docker-setup lab` → installs Docker,
    writes SSH alias + Docker context, idempotent on re-run.
26. `rusta --verbose <any>` → verbose logging.
27. `rusta --log /tmp/x.log <any>` → entire run tee'd to the file.
28. Non-arm64 host → exit 1 before any Tart calls.
29. Missing `brew` → exit 1.
30. VM-not-found (explicit name) → exit 2 with a clear message.
