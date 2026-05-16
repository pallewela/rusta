# Quick Start

```sh
# Create + provision a default Ubuntu 24.04 VM
rusta create

# Boot it (headless by default)
rusta up

# SSH in
rusta ssh

# Find its IP
rusta ip

# Shut it down gracefully
rusta down
```

The first time you run an argument-less command with more than one VM
present, `rusta` interactively prompts you to pick a default and persists
the choice to `~/.local/share/rusta/state.toml`.

## At a glance

| Subcommand                     | Purpose                                                          |
| ------------------------------ | ---------------------------------------------------------------- |
| `rusta up [<vm>]`              | Start a VM (headless by default; `--graphical` for a window).    |
| `rusta down [<vm>]`            | Gracefully shut down a VM (`--force` to hard-stop).              |
| `rusta create [<vm>]`          | Create and provision a new Ubuntu VM.                            |
| `rusta delete <vm>`            | Delete a VM (requires confirmation or `--yes`).                  |
| `rusta list`                   | List Tart VMs and indicate the current default.                  |
| `rusta versions`               | List available Ubuntu OCI tags from `ghcr.io/cirruslabs/ubuntu`. |
| `rusta default [<vm>]`         | Print or set the default VM.                                     |
| `rusta ip [<vm>]`              | Print the guest IP of the VM.                                    |
| `rusta ssh [<vm>] [-- cmd...]` | Open an SSH session or run a command on the VM.                  |
| `rusta docker-setup [<vm>]`    | Install Docker in the VM and wire host SSH/Docker context.       |
| `rusta ssh-copy [<vm>]`        | Copy host `~/.ssh/id_*` and `*.pem` into the VM.                 |

Global flags accepted by every subcommand:

- `--verbose` — verbose logging.
- `--log <file>` — tee all stdout/stderr to the given file.
- `--help`, `-h` — print help and exit.

For full behavior, flags, and exit codes, see the [Commands](commands.md)
reference.

## State

`rusta` keeps a small amount of host-side state under
`~/.local/share/rusta/`:

- `state.toml` — the default VM name.
- `run/<vm>.pid` — PID of the headless `tart run` process.
- `provision/<vm>.sh` — the generated provisioning script (kept for
  debugging).
