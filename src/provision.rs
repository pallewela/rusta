use crate::io as rio;

#[derive(Clone, Debug)]
pub struct Spec<'a> {
    pub ubuntu_version: &'a str,
    pub gui: Option<&'a str>,
}

/// Map known affected releases to their codename for the per-release apt-cache fix.
/// Data-driven: extend this list to cover additional releases.
fn codename_for_release(ver: &str) -> Option<&'static str> {
    match ver {
        "24.04" => Some("noble"),
        // Extend this match when a new release ships with stale ARM64 apt cache files.
        // Example: "26.04" => Some("<codename>"),
        _ => None,
    }
}

pub fn display_manager_for(gui: &str) -> Option<&'static str> {
    match gui {
        "ubuntu-desktop" => Some("gdm3"),
        "xubuntu-desktop" => Some("lightdm"),
        "lubuntu-desktop" => Some("sddm"),
        "lightdm" => Some("lightdm"),
        _ => None,
    }
}

pub fn generate(spec: &Spec) -> String {
    let verbose_xtrace = if rio::verbose() { "set -x" } else { "" };

    let apt_cache_fix = match codename_for_release(spec.ubuntu_version) {
        Some(codename) => format!(
            r#"
# Per-release apt cache fix: remove stale ARM64 cache files that prevent apt
# from resolving dependencies against {codename}-updates / {codename}-security.
echo ">>> Removing stale apt cache files for {codename}-updates/{codename}-security..."
sudo rm -f \
  /var/lib/apt/lists/ports.ubuntu.com_ubuntu-ports_dists_{codename}-updates_main_binary-arm64_Packages \
  /var/lib/apt/lists/ports.ubuntu.com_ubuntu-ports_dists_{codename}-security_main_binary-arm64_Packages
"#
        ),
        None => String::new(),
    };

    let (apt_extra, nm_pre, desktop_systemd) = match spec.gui {
        Some(pkg) => {
            let dm = display_manager_for(pkg).unwrap_or("lightdm");
            let extra = format!("{pkg} {dm}");
            let nm = r#"
# Pre-create NetworkManager config BEFORE installing the desktop meta-package.
echo ">>> Pre-configuring NetworkManager to manage all devices..."
sudo mkdir -p /etc/NetworkManager/conf.d
cat <<'NMEOF' | sudo tee /etc/NetworkManager/conf.d/10-manage-all.conf
[keyfile]
unmanaged-devices=none
NMEOF
"#
            .to_string();
            let post = format!(
                r#"
sudo systemctl restart NetworkManager 2>/dev/null || true
echo ">>> Disabling systemd-networkd-wait-online (NM manages networking now)..."
sudo systemctl disable systemd-networkd-wait-online.service 2>/dev/null || true

echo ">>> Configuring graphical login ({dm})..."
sudo systemctl set-default graphical.target
sudo systemctl enable {dm}

echo ">>> Disabling GNOME Initial Setup welcome screen..."
mkdir -p ~/.config
echo yes > ~/.config/gnome-initial-setup-done
"#
            );
            (extra, nm, post)
        }
        None => (String::new(), String::new(), String::new()),
    };

    let install_line = if apt_extra.is_empty() {
        r#"sudo -E apt-fast install "${APT_OPTS[@]}" spice-vdagent spice-webdavd curl wget git"#.to_string()
    } else {
        format!(
            r#"sudo -E apt-fast install "${{APT_OPTS[@]}}" spice-vdagent spice-webdavd curl wget git {apt_extra}"#
        )
    };

    format!(
        r##"#!/usr/bin/env bash
set -euo pipefail
{verbose_xtrace}

sudo touch /var/log/provision.log
sudo chmod 666 /var/log/provision.log
exec > >(tee -a /var/log/provision.log) 2>&1

export DEBIAN_FRONTEND=noninteractive
export DEBCONF_NONINTERACTIVE_SEEN=true
export NEEDRESTART_MODE=l
export LC_ALL=C.UTF-8
export LANG=C.UTF-8

echo 'debconf debconf/frontend select Noninteractive' | sudo debconf-set-selections

wait_for_dpkg_lock() {{
  local n=0 busy
  while true; do
    busy=0
    if command -v fuser >/dev/null 2>&1; then
      sudo fuser /var/lib/dpkg/lock-frontend >/dev/null 2>&1 && busy=1
      sudo fuser /var/lib/dpkg/lock >/dev/null 2>&1 && busy=1
    fi
    pgrep -x apt-get >/dev/null && busy=1
    pgrep -x dpkg >/dev/null && busy=1
    pgrep -f unattended-upgrade >/dev/null && busy=1
    if [[ "$busy" -eq 0 ]]; then
      return 0
    fi
    echo ">>> Waiting for apt/dpkg lock (unattended-upgrades or another apt)..."
    sleep 5
    n=$((n + 1))
    if [[ $n -gt 120 ]]; then
      echo ">>> Timed out waiting for package manager lock"
      exit 1
    fi
  done
}}

sudo systemctl stop unattended-upgrades.service 2>/dev/null || true
sudo systemctl stop apt-daily.service 2>/dev/null || true
sudo systemctl stop apt-daily-upgrade.service 2>/dev/null || true
sudo systemctl stop apt-daily.timer 2>/dev/null || true
sudo systemctl stop apt-daily-upgrade.timer 2>/dev/null || true

wait_for_dpkg_lock
{apt_cache_fix}

echo ">>> Updating package lists..."
sudo -E apt-get update -y --fix-missing

wait_for_dpkg_lock

echo ">>> Installing apt-fast for parallel downloads..."
sudo -E apt-get install -y software-properties-common
sudo -E add-apt-repository -y ppa:apt-fast/stable
sudo -E apt-get update -y
sudo -E apt-get install -y apt-fast

wait_for_dpkg_lock
{nm_pre}

echo ">>> Installing packages (SPICE tools and extras)..."
APT_OPTS=(-y -o Dpkg::Options::="--force-confdef" -o Dpkg::Options::="--force-confold")
{install_line}

sudo systemctl start spice-vdagent.socket 2>/dev/null || true
sudo systemctl start spice-vdagent.service 2>/dev/null || true
{desktop_systemd}

echo ""
echo ">>> Provisioning complete!"
"##
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_manager_table() {
        assert_eq!(display_manager_for("ubuntu-desktop"), Some("gdm3"));
        assert_eq!(display_manager_for("xubuntu-desktop"), Some("lightdm"));
        assert_eq!(display_manager_for("lubuntu-desktop"), Some("sddm"));
        assert_eq!(display_manager_for("lightdm"), Some("lightdm"));
        assert_eq!(display_manager_for("kde"), None);
    }

    #[test]
    fn codename_only_for_affected_releases() {
        assert_eq!(codename_for_release("24.04"), Some("noble"));
        assert_eq!(codename_for_release("22.04"), None);
        assert_eq!(codename_for_release("20.04"), None);
    }

    #[test]
    fn generate_for_24_04_includes_apt_cache_fix() {
        let s = generate(&Spec { ubuntu_version: "24.04", gui: None });
        assert!(s.contains("Removing stale apt cache files for noble"));
        assert!(s.contains("spice-vdagent"));
        assert!(!s.contains("ubuntu-desktop"));
    }

    #[test]
    fn generate_for_22_04_skips_apt_cache_fix() {
        let s = generate(&Spec { ubuntu_version: "22.04", gui: None });
        assert!(!s.contains("apt cache files"));
        assert!(s.contains("spice-vdagent"));
    }

    #[test]
    fn generate_with_gui_installs_desktop_and_dm() {
        let s = generate(&Spec { ubuntu_version: "24.04", gui: Some("ubuntu-desktop") });
        assert!(s.contains("ubuntu-desktop gdm3"));
        assert!(s.contains("Pre-configuring NetworkManager"));
        assert!(s.contains("Disabling GNOME Initial Setup"));
    }

    #[test]
    fn generate_with_xubuntu_uses_lightdm() {
        let s = generate(&Spec { ubuntu_version: "22.04", gui: Some("xubuntu-desktop") });
        assert!(s.contains("xubuntu-desktop lightdm"));
    }

    #[test]
    fn generate_with_lubuntu_uses_sddm() {
        let s = generate(&Spec { ubuntu_version: "22.04", gui: Some("lubuntu-desktop") });
        assert!(s.contains("lubuntu-desktop sddm"));
    }

    #[test]
    fn generate_without_gui_omits_nm_block() {
        let s = generate(&Spec { ubuntu_version: "22.04", gui: None });
        assert!(!s.contains("Pre-configuring NetworkManager"));
        assert!(!s.contains("graphical.target"));
    }
}
