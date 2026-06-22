//! Shared test harness for integration tests.
//!
//! Each test gets its own tempdir with:
//!   - A fake `tart` shell script that maintains a per-test VM list in a text file.
//!   - All other external binaries (`sshpass`, `ssh`, `scp`, `ssh-copy-id`,
//!     `ssh-keygen`, `brew`) redirected to `/usr/bin/true`.
//!   - A fake `docker` binary that fails `context inspect` so `context create`
//!     runs (covering both branches of the docker context code).
//!   - `RUSTA_SKIP_PREFLIGHT=1` so the arm64/brew/tart checks don't fire.
//!   - `RUSTA_STATE_ROOT` and `RUSTA_SSH_DIR` pointing into the tempdir.
//!   - `RUSTA_POLL_MS=10` so any `wait_for_*` retries don't slow the suite.

#![allow(dead_code)]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};
use std::thread;

use tempfile::TempDir;

/// A tiny single-thread HTTP server mimicking ghcr.io for `versions`/`create`
/// resolution. Responds to two paths:
///   GET …/token → the supplied token body
///   GET …/tags  → the supplied tags body
pub struct MockGhcr {
    addr: String,
    _handle: thread::JoinHandle<()>,
    _stop: Arc<Mutex<bool>>,
}

impl MockGhcr {
    pub fn start(tags_body: &'static str, token_body: &'static str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = format!("http://{}", listener.local_addr().unwrap());
        let stop = Arc::new(Mutex::new(false));
        let stop_c = stop.clone();
        let h = thread::spawn(move || {
            listener.set_nonblocking(false).unwrap();
            // Serve plenty of requests for a multi-source call.
            for _ in 0..40 {
                if *stop_c.lock().unwrap() {
                    break;
                }
                let (mut sock, _) = match listener.accept() {
                    Ok(p) => p,
                    Err(_) => continue,
                };
                let mut buf = [0u8; 4096];
                let n = sock.read(&mut buf).unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let body = if req.contains("/token") {
                    token_body
                } else {
                    tags_body
                };
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = sock.write_all(resp.as_bytes());
            }
        });
        Self { addr, _handle: h, _stop: stop }
    }

    pub fn token_url(&self) -> String {
        format!("{}/token", self.addr)
    }

    pub fn tags_url(&self) -> String {
        format!("{}/tags", self.addr)
    }
}

pub struct Harness {
    pub _dir: TempDir,
    pub root: PathBuf,
    pub bin_dir: PathBuf,
    pub state_root: PathBuf,
    pub ssh_dir: PathBuf,
    pub tart_state: PathBuf,
}

impl Harness {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().to_path_buf();
        let bin_dir = root.join("bin");
        let state_root = root.join("state");
        let ssh_dir = root.join("ssh");
        let tart_state = root.join("tart");
        for p in [&bin_dir, &state_root, &ssh_dir, &tart_state] {
            std::fs::create_dir_all(p).unwrap();
        }
        std::fs::write(tart_state.join("vms.txt"), b"").unwrap();
        write_fake_tart(&bin_dir.join("fake-tart"));
        write_fake_docker(&bin_dir.join("fake-docker"));
        Self { _dir: dir, root, bin_dir, state_root, ssh_dir, tart_state }
    }

    pub fn add_vm(&self, name: &str, state: &str) {
        let path = self.tart_state.join("vms.txt");
        let s = std::fs::read_to_string(&path).unwrap_or_default();
        let mut out = String::new();
        for line in s.lines() {
            let first = line.split_whitespace().next().unwrap_or("");
            if !first.is_empty() && first != name {
                out.push_str(line);
                out.push('\n');
            }
        }
        out.push_str(&format!("{name} {state}\n"));
        std::fs::write(path, out).unwrap();
    }

    /// Returns the full argv that the fake `tart` last saw for a `tart run …`
    /// invocation, or `None` if no `tart run` has happened yet in this harness.
    /// Each line of `run.log` is one whitespace-joined argv (e.g. `run lab --no-graphics`).
    pub fn last_run_args(&self) -> Option<Vec<String>> {
        let s = std::fs::read_to_string(self.tart_state.join("run.log")).ok()?;
        let last = s.lines().filter(|l| !l.is_empty()).last()?;
        Some(last.split_whitespace().map(str::to_string).collect())
    }

    pub fn vm_state(&self, name: &str) -> Option<String> {
        let s = std::fs::read_to_string(self.tart_state.join("vms.txt")).ok()?;
        for line in s.lines() {
            let mut it = line.split_whitespace();
            let n = it.next()?;
            let st = it.next().unwrap_or("");
            if n == name {
                return Some(st.to_string());
            }
        }
        None
    }

    pub fn write_dummy_ssh_key(&self) {
        let priv_path = self.ssh_dir.join("id_ed25519");
        let pub_path = self.ssh_dir.join("id_ed25519.pub");
        std::fs::write(&priv_path, b"FAKE PRIVATE KEY\n").unwrap();
        std::fs::write(&pub_path, b"ssh-ed25519 FAKE\n").unwrap();
    }

    pub fn write_pem(&self, name: &str) {
        std::fs::write(self.ssh_dir.join(name), b"FAKE PEM\n").unwrap();
    }

    pub fn cmd(&self, args: &[&str]) -> Command {
        let exe = env!("CARGO_BIN_EXE_rusta");
        let mut c = Command::new(exe);
        c.args(args);
        // Don't env_clear: cargo-llvm-cov sets LLVM_PROFILE_FILE in the parent env
        // and the spawned rusta binary must inherit it to record coverage. Wipe only
        // the RUSTA_* vars that could leak in, then set our explicit overrides.
        for (k, _) in std::env::vars_os() {
            if let Some(s) = k.to_str() {
                if s.starts_with("RUSTA_") {
                    c.env_remove(s);
                }
            }
        }
        c.env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin");
        c.env("HOME", &self.root);
        c.env("RUSTA_SKIP_PREFLIGHT", "1");
        // Block the background update check unless a specific test
        // explicitly opts in by unsetting this env var.
        c.env("RUSTA_NO_UPDATE_CHECK", "1");
        c.env("RUSTA_STATE_ROOT", &self.state_root);
        c.env("RUSTA_SSH_DIR", &self.ssh_dir);
        c.env("RUSTA_TART_BIN", self.bin_dir.join("fake-tart"));
        c.env("RUSTA_SSHPASS_BIN", "true");
        c.env("RUSTA_SSH_BIN", "true");
        c.env("RUSTA_SCP_BIN", "true");
        c.env("RUSTA_SSH_COPY_ID_BIN", "true");
        c.env("RUSTA_BREW_BIN", "true");
        c.env("RUSTA_SSH_KEYGEN_BIN", "true");
        c.env("RUSTA_UNAME_BIN", "true");
        c.env("RUSTA_DOCKER_BIN", self.bin_dir.join("fake-docker"));
        c.env("RUSTA_POLL_MS", "10");
        c.env("RUSTA_MAX_TIMEOUT_S", "2");
        c.env("RUSTA_FAKE_TART_STATE", &self.tart_state);
        c
    }

    pub fn run(&self, args: &[&str]) -> Output {
        self.cmd(args).output().expect("spawn rusta")
    }
}

pub fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).to_string()
}

pub fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).to_string()
}

pub fn code(o: &Output) -> i32 {
    o.status.code().unwrap_or(-1)
}

fn write_executable(path: &Path, contents: &str) {
    std::fs::write(path, contents).unwrap();
    let mut perms = std::fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms).unwrap();
}

fn write_fake_tart(path: &Path) {
    let script = r##"#!/usr/bin/env bash
set -u
S="${RUSTA_FAKE_TART_STATE:?missing RUSTA_FAKE_TART_STATE}"
LIST="$S/vms.txt"
touch "$LIST"

vm_set_state() {
  local name="$1" state="$2" tmp
  tmp="$(mktemp)"
  local found=false
  while IFS=' ' read -r n st; do
    [ -z "$n" ] && continue
    if [ "$n" = "$name" ]; then
      echo "$name $state" >> "$tmp"
      found=true
    else
      echo "$n $st" >> "$tmp"
    fi
  done < "$LIST"
  if [ "$found" = false ]; then
    echo "$name $state" >> "$tmp"
  fi
  mv "$tmp" "$LIST"
}

vm_remove() {
  local name="$1" tmp
  tmp="$(mktemp)"
  while IFS=' ' read -r n st; do
    [ -z "$n" ] && continue
    if [ "$n" != "$name" ]; then
      echo "$n $st" >> "$tmp"
    fi
  done < "$LIST"
  mv "$tmp" "$LIST"
}

emit_list_json() {
  printf '['
  local first=true
  while IFS=' ' read -r n st; do
    [ -z "$n" ] && continue
    if [ "$first" = false ]; then printf ','; fi
    first=false
    local running='false'
    [ "$st" = "running" ] && running='true'
    printf '{"Source":"local","Name":"%s","State":"%s","Running":%s,"Disk":80,"Size":5}' "$n" "$st" "$running"
  done < "$LIST"
  printf ']\n'
}

cmd="${1:-}"
shift || true
case "$cmd" in
  list)
    emit_list_json
    ;;
  ip)
    echo "192.168.64.10"
    ;;
  clone)
    vm_set_state "$2" stopped
    ;;
  set)
    exit 0
    ;;
  delete)
    vm_remove "$1"
    ;;
  stop)
    vm_set_state "$1" stopped
    ;;
  run)
    echo "run $*" >> "$S/run.log"
    vm_set_state "$1" running
    ;;
  exec)
    vm=""
    if [ "${1:-}" = "-i" ]; then
      shift
      vm="$1"
      shift
      cat > /dev/null
    else
      vm="$1"
      shift
    fi
    cur_state=""
    while IFS=' ' read -r n st; do
      if [ "$n" = "$vm" ]; then cur_state="$st"; break; fi
    done < "$LIST"
    if [ "$cur_state" != "running" ]; then
      exit 1
    fi
    if printf '%s\n' "$@" | grep -q shutdown; then
      vm_set_state "$vm" stopped
    fi
    exit 0
    ;;
  --version)
    echo "tart 9.9-fake"
    ;;
  *)
    echo "fake-tart: unknown subcommand: $cmd $*" >&2
    exit 1
    ;;
esac
"##;
    write_executable(path, script);
}

/// Write a fake sshpass that succeeds when invoked for the readiness probe
/// (`ssh ... user@host true`) but exits 1 for anything else. This lets
/// failure-path tests reach the real ssh call without waiting on
/// wait_for_ssh's full 120s timeout.
pub fn write_sshpass_probe_ok_else_fail(path: &Path) {
    let script = r#"#!/usr/bin/env bash
last="${@: -1}"
if [ "$last" = "true" ]; then exit 0; fi
exit 1
"#;
    write_executable(path, script);
}

fn write_fake_docker(path: &Path) {
    let script = r#"#!/usr/bin/env bash
case "${1:-} ${2:-}" in
  "context inspect")
    exit 1
    ;;
  "context create")
    exit 0
    ;;
  *)
    exit 0
    ;;
esac
"#;
    write_executable(path, script);
}
