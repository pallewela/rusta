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
- Boot mode follows the VM's `create`-time `--gui` choice. VMs created
  without `--gui` boot headless (`tart run <vm> --no-graphics`).
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
| `--version <ver>`       | `24.04`                | Ubuntu release line (OCI tag on `ghcr.io/cirruslabs/ubuntu`).            |
| `--gui [pkg]`           | off / `ubuntu-desktop` | Install a desktop (`ubuntu-desktop`, `xubuntu-desktop`, `lubuntu-desktop`, `lightdm`). |
| `--cpus <n>`            | `6`                    | CPU count.                                                               |
| `--memory <mb>`         | `8192`                 | Memory in MB.                                                            |
| `--disk <gb>`           | `80`                   | Disk size in GB.                                                         |
| `--user <username>`     | `admin`                | Guest login username (image-dependent).                                  |
| `--password <password>` | `admin`                | Guest login password used by `sshpass`.                                  |
| `--ssh-copy-keys`       | off                    | After provisioning, copy host SSH keys into the guest.                   |
| `--debug-no-headless`   | off                    | Run with a graphics window during provisioning (debug only).             |

`rusta create` never assumes a name: if `<vm>` is omitted and stdin is a
TTY it prompts (suggesting `ubuntu-<version>` based on `--version`); a
non-TTY stdin exits 1. The chosen name is **not** written to
`state.default_vm`.

If the VM name already exists, creation is skipped and a recreate hint is
printed. Otherwise, `rusta` clones the OCI image, sets CPU/memory/disk,
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
rusta versions
```

List available Ubuntu OCI tags from `ghcr.io/cirruslabs/ubuntu`:

1. Fetch an anonymous pull token from `ghcr.io/token`.
2. List tags from `ghcr.io/v2/cirruslabs/ubuntu/tags/list`.
3. Filter to tags matching `^\d+\.\d+$`, sort ascending, print one per
   line.
4. Highlight `24.04` as `(default)`.

Token/list failures are fatal (exit 1).

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
