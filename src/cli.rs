use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "rusta",
    version,
    about = "macOS CLI for managing Ubuntu VMs on Apple Silicon via Tart",
    arg_required_else_help = false,
    subcommand_required = false
)]
pub struct Cli {
    /// Verbose logging
    #[arg(long, global = true)]
    pub verbose: bool,

    /// Tee all stdout/stderr to the given file
    #[arg(long, value_name = "FILE", global = true)]
    pub log: Option<String>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Start a VM (headless by default)
    Up(UpArgs),
    /// Gracefully shut down a VM (--force to hard-stop)
    Down(DownArgs),
    /// Create + provision a new Ubuntu VM
    Create(CreateArgs),
    /// Delete a VM (Tart state). Requires confirmation or --yes
    Delete(DeleteArgs),
    /// List Tart VMs and indicate the current default
    List,
    /// List available Ubuntu OCI tags from ghcr.io/cirruslabs/ubuntu
    Versions,
    /// Print or set the default VM
    Default(DefaultArgs),
    /// Print the guest IP of the VM
    Ip(VmOnlyArgs),
    /// Open an SSH session (or run a command) on the VM
    Ssh(SshArgs),
    /// Install Docker in the VM and wire host SSH/Docker context
    DockerSetup(VmOnlyArgs),
    /// Copy host ~/.ssh/id_* and *.pem into the VM
    SshCopy(VmOnlyArgs),
    /// Set whether `rusta up <vm>` boots with a graphics window by default
    SetGui(SetGuiArgs),
    /// Print a shell completion script to stdout
    #[command(hide = true)]
    Completions(CompletionsArgs),
    /// Generate man pages into the given directory
    #[command(hide = true)]
    Man(ManArgs),
}

#[derive(Args, Debug)]
pub struct CompletionsArgs {
    /// Target shell
    pub shell: clap_complete::Shell,
}

#[derive(Args, Debug)]
pub struct ManArgs {
    /// Output directory (created if absent). Writes `rusta.1` plus one page per subcommand.
    pub dir: std::path::PathBuf,
}

#[derive(Args, Debug)]
pub struct UpArgs {
    pub vm: Option<String>,
    /// Force a graphics window (overrides per-VM default)
    #[arg(
        long = "graphical",
        short = 'G',
        visible_aliases = ["graphics", "gui"],
        conflicts_with = "no_gui",
    )]
    pub graphical: bool,
    /// Force headless boot, even if the VM was created with --gui
    #[arg(long = "no-gui", visible_alias = "no-graphics")]
    pub no_gui: bool,
}

#[derive(Args, Debug)]
pub struct DownArgs {
    pub vm: Option<String>,
    /// Hard-stop the VM instead of graceful shutdown
    #[arg(long, short = 'f')]
    pub force: bool,
    /// Graceful shutdown timeout in seconds
    #[arg(long, default_value_t = 60)]
    pub timeout: u64,
}

#[derive(Args, Debug)]
pub struct CreateArgs {
    pub vm: Option<String>,
    /// Ubuntu release line (OCI tag)
    #[arg(long, default_value = "24.04")]
    pub version: String,
    /// Install a desktop. Allowed: ubuntu-desktop, xubuntu-desktop, lubuntu-desktop, lightdm
    #[arg(long, num_args = 0..=1, default_missing_value = "ubuntu-desktop", value_name = "PKG")]
    pub gui: Option<String>,
    #[arg(long, default_value_t = 6)]
    pub cpus: u32,
    /// Memory in MB
    #[arg(long, default_value_t = 8192)]
    pub memory: u32,
    /// Disk size in GB
    #[arg(long, default_value_t = 80)]
    pub disk: u32,
    #[arg(long, default_value = "admin")]
    pub user: String,
    #[arg(long, default_value = "admin")]
    pub password: String,
    /// After provisioning, copy host SSH keys into the guest
    #[arg(long)]
    pub ssh_copy_keys: bool,
    /// Run with a graphics window during provisioning (debug only)
    #[arg(long)]
    pub debug_no_headless: bool,
}

#[derive(Args, Debug)]
pub struct DeleteArgs {
    pub vm: String,
    #[arg(long, short = 'y')]
    pub yes: bool,
    /// Stop the VM if running, then delete
    #[arg(long)]
    pub force_running: bool,
}

#[derive(Args, Debug)]
pub struct DefaultArgs {
    pub vm: Option<String>,
}

#[derive(Args, Debug)]
pub struct VmOnlyArgs {
    pub vm: Option<String>,
}

#[derive(Args, Debug)]
pub struct SshArgs {
    pub vm: Option<String>,
    /// Boot the VM if it's not running
    #[arg(long)]
    pub auto_up: bool,
    /// Remote command (after `--`)
    #[arg(last = true)]
    pub remote: Vec<String>,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuiMode {
    /// `rusta up <vm>` boots with a graphics window by default
    On,
    /// `rusta up <vm>` boots headlessly by default
    Off,
}

#[derive(Args, Debug)]
pub struct SetGuiArgs {
    /// VM whose recorded `gui` preference will be updated
    pub vm: String,
    /// Desired default boot mode for this VM
    pub mode: GuiMode,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_create_defaults() {
        let cli = Cli::try_parse_from(["rusta", "create"]).unwrap();
        let Some(Command::Create(a)) = cli.command else { panic!("expected create") };
        assert_eq!(a.version, "24.04");
        assert_eq!(a.cpus, 6);
        assert_eq!(a.memory, 8192);
        assert_eq!(a.disk, 80);
        assert_eq!(a.user, "admin");
        assert!(a.gui.is_none());
        assert!(!a.ssh_copy_keys);
    }

    #[test]
    fn parses_create_with_gui_no_arg_defaults_to_ubuntu_desktop() {
        let cli = Cli::try_parse_from(["rusta", "create", "--gui"]).unwrap();
        let Some(Command::Create(a)) = cli.command else { panic!("expected create") };
        assert_eq!(a.gui.as_deref(), Some("ubuntu-desktop"));
    }

    #[test]
    fn parses_create_with_explicit_gui() {
        let cli = Cli::try_parse_from(["rusta", "create", "--gui", "xubuntu-desktop"]).unwrap();
        let Some(Command::Create(a)) = cli.command else { panic!("expected create") };
        assert_eq!(a.gui.as_deref(), Some("xubuntu-desktop"));
    }

    #[test]
    fn parses_up_with_graphical_flag() {
        let cli = Cli::try_parse_from(["rusta", "up", "lab", "--graphical"]).unwrap();
        let Some(Command::Up(a)) = cli.command else { panic!("expected up") };
        assert_eq!(a.vm.as_deref(), Some("lab"));
        assert!(a.graphical);
        assert!(!a.no_gui);
    }

    #[test]
    fn up_graphical_has_aliases() {
        for flag in ["--graphical", "-G", "--graphics", "--gui"] {
            let cli = Cli::try_parse_from(["rusta", "up", "lab", flag]).unwrap();
            let Some(Command::Up(a)) = cli.command else { panic!("expected up: {flag}") };
            assert!(a.graphical, "{flag} should set graphical");
            assert!(!a.no_gui, "{flag} should leave no_gui false");
        }
    }

    #[test]
    fn up_no_gui_has_alias() {
        for flag in ["--no-gui", "--no-graphics"] {
            let cli = Cli::try_parse_from(["rusta", "up", "lab", flag]).unwrap();
            let Some(Command::Up(a)) = cli.command else { panic!("expected up: {flag}") };
            assert!(a.no_gui, "{flag} should set no_gui");
            assert!(!a.graphical, "{flag} should leave graphical false");
        }
    }

    #[test]
    fn up_graphical_and_no_gui_conflict() {
        assert!(Cli::try_parse_from(["rusta", "up", "lab", "--graphical", "--no-gui"]).is_err());
        assert!(Cli::try_parse_from(["rusta", "up", "lab", "--gui", "--no-graphics"]).is_err());
    }

    #[test]
    fn parses_down_with_timeout_and_force() {
        let cli = Cli::try_parse_from(["rusta", "down", "--force", "--timeout", "5"]).unwrap();
        let Some(Command::Down(a)) = cli.command else { panic!("expected down") };
        assert!(a.force);
        assert_eq!(a.timeout, 5);
        assert!(a.vm.is_none());
    }

    #[test]
    fn parses_ssh_with_remote_command_after_dashdash() {
        let cli = Cli::try_parse_from(["rusta", "ssh", "lab", "--", "uname", "-a"]).unwrap();
        let Some(Command::Ssh(a)) = cli.command else { panic!("expected ssh") };
        assert_eq!(a.vm.as_deref(), Some("lab"));
        assert_eq!(a.remote, vec!["uname".to_string(), "-a".to_string()]);
    }

    #[test]
    fn parses_global_verbose_and_log() {
        let cli = Cli::try_parse_from(["rusta", "--verbose", "--log", "/tmp/x", "list"]).unwrap();
        assert!(cli.verbose);
        assert_eq!(cli.log.as_deref(), Some("/tmp/x"));
    }

    #[test]
    fn no_subcommand_is_ok() {
        let cli = Cli::try_parse_from(["rusta"]).unwrap();
        assert!(cli.command.is_none());
    }

    #[test]
    fn delete_requires_vm() {
        assert!(Cli::try_parse_from(["rusta", "delete"]).is_err());
    }

    #[test]
    fn delete_yes_short_flag() {
        let cli = Cli::try_parse_from(["rusta", "delete", "lab", "-y"]).unwrap();
        let Some(Command::Delete(a)) = cli.command else { panic!() };
        assert_eq!(a.vm, "lab");
        assert!(a.yes);
    }
}
