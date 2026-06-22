# Commands

`rusta` is invoked as:

```
rusta <command> [args...] [flags...]
```

## Global flags

Accepted by every subcommand:

| Flag           | Default | Description                                                          |
| -------------- | ------- | -------------------------------------------------------------------- |
| `--verbose`    | off     | Verbose logging (equivalent to `set -x` + `LogLevel=INFO` on SSH).   |
| `--log <file>` | —       | Tee all stdout/stderr to the given file.                             |
| `--help`, `-h` | —       | Print help for the current (sub)command and exit 0.                  |

`rusta` (no args) and `rusta --help` print top-level help and exit 0.

## Exit codes

| Code | Meaning                                                                              |
| ---- | ------------------------------------------------------------------------------------ |
| 0    | Success (including `--help`, `versions`, no-op when target already in desired state).|
| 1    | Bad usage, unmet prerequisite, validation failure, or runtime error.                 |
| 2    | VM not found (when an explicit name was given or default resolution failed).         |

---

## `rusta up`

```
rusta up [<vm>] [--graphical|-G|--graphics|--gui] [--no-gui|--no-graphics]
```

Boot a VM.

- Resolves `<vm>` from the default VM if omitted.
- If the VM is already running, prints `[skip]` and exits 0.
- Boots with a graphics window by default. VMs explicitly created without
  `--gui` boot headless (`tart run <vm> --no-graphics`); VMs created with
  `--gui` or VMs with no recorded preference boot graphical.
- `--graphical` (aliases: `-G`, `--graphics`, `--gui`) forces a graphics
  window for this invocation.
- `--no-gui` (alias: `--no-graphics`) forces headless boot, even for
  GUI-enabled VMs.
- `--graphical` and `--no-gui` are mutually exclusive.
- Backgrounds the run and writes the PID to
  `~/.local/share/rusta/run/<vm>.pid`.
- Waits for the Tart guest agent and prints the guest IP once available
  (best-effort).

## `rusta down`

```
rusta down [<vm>] [--force|-f] [--timeout <secs>]
```

Stop a VM.

- Resolves `<vm>` if omitted; prints `[skip]` if already stopped.
- **Graceful (default):** issues `sudo shutdown -h now` via `tart exec` and
  waits up to `--timeout` seconds (default **60s**) for the `tart run`
  process to exit. If the timeout expires, exits 1 with a hint to retry
  with `--force`.
- **`--force`:** skip the graceful path; call `tart stop <vm>` (or kill the
  recorded PID).
- Removes the stale `~/.local/share/rusta/run/<vm>.pid` on success.

## `rusta create`

```
rusta create [<vm>] [flags]
```

Clone + provision a new Ubuntu VM.

| Flag                    | Default                | Description                                                              |
| ----------------------- | ---------------------- | ------------------------------------------------------------------------ |
| `--version <ver>`       | `24.04`                | Ubuntu release line (OCI tag), resolved across configured sources.       |
| `--image <name>`        | first image            | Image family (repo) to clone, e.g. `ubuntu-desktop`; defaults to the first configured image. Composes with `--source`. |
| `--gui [pkg]`           | off / `ubuntu-desktop` | Install a desktop (`ubuntu-desktop`, `xubuntu-desktop`, `lubuntu-desktop`, `lightdm`). |
| `--cpus <n>`            | `6`                    | CPU count.                                                               |
| `--memory <mb>`         | `8192`                 | Memory in MB.                                                            |
| `--disk <gb>`           | `80`                   | Disk size in GB.                                                         |
| `--user <username>`     | `admin`                | Guest login username (image-dependent).                                  |
| `--password <password>` | `admin`                | Guest login password used by `sshpass`.                                  |
| `--ssh-copy-keys`       | off                    | After provisioning, copy host SSH keys into the guest.                   |
| `--debug-no-headless`   | off                    | Run with a graphics window during provisioning (debug only).             |
| `--source <registry>`   | (all sources)          | Pin resolution to one configured source (registry prefix or label).      |
| `--image-ref <ref>`     | (unset)                | Clone this exact image reference verbatim, bypassing sources and images. Conflicts with `--source` and `--image`. |

`rusta create` never assumes a name: if `<vm>` is omitted and stdin is a
TTY it prompts (suggesting `<image>-<version>`, e.g. `ubuntu-2404` or
`ubuntu-desktop-2404`); a non-TTY stdin exits 1. The chosen name is **not**
written to `state.default_vm`.

If the VM name already exists, creation is skipped and a recreate hint is
printed. Otherwise, `rusta` resolves which image to clone (the selected
[image](#rusta-image) across configured [sources](#rusta-source)), clones it,
sets CPU/memory/disk,
generates a provisioning script under
`~/.local/share/rusta/provision/<vm>.sh`, boots, runs the script, and
shuts the VM down.

## `rusta delete`

```
rusta delete <vm> [--yes|-y] [--force-running]
```

Delete a VM from Tart's storage.

- Requires an explicit `<vm>` (no default-VM fallback).
- Refuses to run if the VM is currently running unless `--force-running`
  is given (stop + delete in one shot).
- Prompts for confirmation unless `--yes` is given.
- Clears `state.default_vm` if it pointed at this VM.

## `rusta list`

```
rusta list
```

Print a table of all Tart VMs with their status and a `*` next to the
resolved default. Exits 0 even if there are no VMs.

```
NAME          STATUS    DEFAULT
ubuntu-2404   running   *
lab-22        stopped
```

## `rusta versions`

```
rusta versions [--source <registry>] [--image <name>]
```

List the available OCI tags across the configured **sources × images** matrix
(see [`rusta source`](#rusta-source) and [`rusta image`](#rusta-image)). For each
`(source, image)` cell, `rusta` fetches a pull token and lists tags from
`<host>/v2/<namespace>/<image>/tags/list`, keeps `X.Y` tags, then merges and
sorts them, highlighting `24.04` as `(default)`.

- With a single source **and** a single image the list is unannotated (legacy
  format).
- With multiple sources but one image, each tag shows which source(s) provide it;
  on a conflict, the one `create` would pick (first by priority) is noted
  (`(create uses <label>)`).
- With multiple images, each tag line groups providers by image:
  `<image>: <src>, <src>  (create uses <src>)`.
- `--source <registry>` limits output to one source; `--image <name>` limits it
  to one image. They compose.
- A source whose **host is unreachable** is skipped with a warning; a source that
  simply **lacks an image** is a silent empty cell. `versions` only fails (exit 1)
  when **no** cell produces any result.

```
$ rusta versions
22.04   ubuntu: cirruslabs, pallewela  (create uses cirruslabs)
24.04 (default)   ubuntu: cirruslabs   ubuntu-desktop: pallewela
25.04   ubuntu-desktop: pallewela
```

## `rusta source`

```
rusta source                              # list (default action)
rusta source add <registry>               # e.g. ghcr.io/pallewela
rusta source rm <registry>
rusta source move <registry> <position>   # 1-based priority
```

Manage the **image sources** that `create` and `versions` consider. A source is a
registry host + namespace prefix (e.g. `ghcr.io/cirruslabs`); rusta appends the
selected [image](#rusta-image) (`ubuntu` by default) to form the repository.
Sources are an ordered list — **position is priority**, and the first source
advertising a requested version wins.

- When no sources are configured, rusta uses a built-in default of
  `ghcr.io/cirruslabs`, preserving the original behavior. Adding your first source
  materializes that default ahead of it, so `create` still finds the stock images.
- `add` validates the prefix (a trailing `/ubuntu` is stripped; a tag is rejected).
  **Only `ghcr.io` is supported for now**; other registries are rejected (you can
  still clone any image once-off with `rusta create --image-ref <ref>`).
- `rm` of the last remaining source re-seeds the default — rusta is never
  sourceless. `rm`/`move` of an unknown source exit 2.

```
$ rusta source add ghcr.io/pallewela
  [ok] Added source: ghcr.io/pallewela
$ rusta source list
==> Image sources (priority order):
  1. ghcr.io/cirruslabs  (cirruslabs)
  2. ghcr.io/pallewela  (pallewela)
```

## `rusta image`

```
rusta image                              # list (default action)
rusta image add <name>                   # e.g. ubuntu-desktop
rusta image rm <name>
rusta image move <name> <position>       # 1-based priority
```

Manage the **image names** (repositories) that rusta clones under each source. An
image is a single repository segment (e.g. `ubuntu`, `ubuntu-desktop`); the
namespace comes from the source. Images are a global, ordered list — **position is
priority**, and the **first image is the `create` default**.

- When no images are configured, rusta uses built-in defaults of `ubuntu` and
  `ubuntu-desktop` (with `ubuntu` as the create default). Adding your first image
  materializes those defaults ahead of it.
- The list is **global** (applied to every source). `rusta create --image <name>`
  picks one for a run; `versions` enumerates the full sources × images matrix.
- `add` validates the name (a single segment: no `/`, no `:` tag, lowercase). A
  source that doesn't host an image is just skipped — not an error.
- `rm` of the last remaining image re-seeds the defaults — rusta is never imageless.
  `rm`/`move` of an unknown image exit 2.

```
$ rusta image list
==> Images (priority order; first is the create default):
  1. ubuntu  (default)
  2. ubuntu-desktop
$ rusta image add ubuntu-kairos
  [ok] Added image: ubuntu-kairos
```

## `rusta default`

```
rusta default [<vm>]
```

- No arg: print the resolved default VM, or `no default set` and exit 1
  (never prompts).
- With arg: set `state.default_vm = <vm>`. Exits 2 if `<vm>` does not
  exist.

## `rusta ip`

```
rusta ip [<vm>]
```

Print `tart ip <vm>` (waits up to 60s). Exits 1 if no IP is obtained.

## `rusta ssh`

```
rusta ssh [<vm>] [--auto-up] [-- cmd args...]
```

- Resolves `<vm>` if omitted.
- If the VM is not running, exits 1 with a hint to `rusta up`. Pass
  `--auto-up` to boot first.
- Connects via `sshpass -p <password> ssh <user>@<ip>` with the standard
  rusta SSH options.
- Anything after `--` is executed as a remote command; otherwise an
  interactive shell is opened.

## `rusta docker-setup`

```
rusta docker-setup [<vm>]
```

Install Docker Engine inside an existing VM and wire host-side
`docker context` + `~/.ssh/config` alias.

- Resolves `<vm>` if omitted; boots the VM if not running and shuts it
  back down at the end (only when `rusta` started it itself).
- Ensures host has `sshpass` and `docker` (auto-installed via Homebrew).
- Generates `~/.ssh/id_ed25519` (empty passphrase) if missing and
  `ssh-copy-id`s the public key into the guest.
- Inside the guest, installs Docker via
  `curl -fsSL https://get.docker.com | sudo sh` only if `docker` is
  absent. Adds `$USER` to the `docker` group and runs
  `systemctl enable --now docker`.
- On host: idempotently appends a `Host docker-<vm>` block to
  `~/.ssh/config` pinned to the observed IP and idempotently creates a
  Docker context `docker-<vm>` pointing at `ssh://<user>@docker-<vm>`.

## `rusta ssh-copy`

```
rusta ssh-copy [<vm>]
```

Copy host `~/.ssh/id_*` and `~/.ssh/*.pem` files into the guest's
`~/.ssh/`.

- Resolves `<vm>` if omitted; boots the VM if not running and shuts it
  back down at the end (only when `rusta` started it itself).
- Exits 1 if the host has no `~/.ssh`.
- If no matching files exist, prints `[skip]` and exits 0.
- In the guest: `mkdir -p ~/.ssh && chmod 700 ~/.ssh`, `scp`s the files,
  normalizes permissions (`*.pub` → 644, others → 600).

## `rusta set-gui`

```
rusta set-gui <vm> <on|off>
```

Update the per-VM `gui` preference stored in `state.toml` for an existing
VM, so that `rusta up <vm>` defaults to the chosen boot mode on future
invocations.

- `on` — `rusta up <vm>` boots with a graphics window by default.
- `off` — `rusta up <vm>` boots headlessly by default.
- Requires an explicit `<vm>`; exits 2 if the VM is not known to Tart.
- Equivalent to the value `rusta create` writes based on whether `--gui`
  was passed. Useful for VMs created before per-VM `gui` tracking
  existed, or to flip an existing VM's default without recreating it.
- Per-invocation `--gui` / `--no-gui` on `rusta up` still override the
  stored preference.
