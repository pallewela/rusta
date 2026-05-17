use std::fs;
use std::path::Path;

use clap::CommandFactory;
use clap_complete::generate;

use crate::cli::{Cli, CompletionsArgs, ManArgs};
use crate::error::{Error, Result};

pub fn completions(args: CompletionsArgs) -> Result<u8> {
    let mut cmd = Cli::command();
    let bin = cmd.get_name().to_string();
    generate(args.shell, &mut cmd, bin, &mut std::io::stdout());
    Ok(0)
}

pub fn man(args: ManArgs) -> Result<u8> {
    write_man_pages(&args.dir, Cli::command())
        .map_err(|e| Error::msg(format!("write man pages: {e}")))?;
    Ok(0)
}

fn write_man_pages(dir: &Path, cmd: clap::Command) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    let bin = cmd.get_name().to_string();
    write_one(dir, &bin, &cmd)?;
    for sub in cmd.get_subcommands() {
        if sub.is_hide_set() {
            continue;
        }
        let full = format!("{bin}-{}", sub.get_name());
        // clap::Command::name requires `'static`. These few small strings are
        // allocated once per `rusta man` invocation, which exits immediately
        // after — so leaking them is fine and avoids a separate name-arena.
        let full_static: &'static str = Box::leak(full.into_boxed_str());
        let sub_named = sub.clone().name(full_static);
        write_one(dir, full_static, &sub_named)?;
    }
    Ok(())
}

fn write_one(dir: &Path, stem: &str, cmd: &clap::Command) -> std::io::Result<()> {
    let path = dir.join(format!("{stem}.1"));
    let mut out = fs::File::create(&path)?;
    clap_mangen::Man::new(cmd.clone()).render(&mut out)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap_complete::Shell;
    use tempfile::tempdir;

    #[test]
    fn man_writes_root_and_subcommand_pages() {
        let d = tempdir().unwrap();
        write_man_pages(d.path(), Cli::command()).unwrap();
        assert!(d.path().join("rusta.1").exists(), "root page missing");
        assert!(d.path().join("rusta-create.1").exists(), "rusta-create page missing");
        assert!(d.path().join("rusta-up.1").exists(), "rusta-up page missing");
        // Hidden subcommands should not get pages.
        assert!(!d.path().join("rusta-completions.1").exists(), "hidden cmd leaked");
        assert!(!d.path().join("rusta-man.1").exists(), "hidden cmd leaked");
    }

    #[test]
    fn completions_emits_zsh_compdef_header() {
        let mut cmd = Cli::command();
        let mut buf = Vec::new();
        generate(Shell::Zsh, &mut cmd, "rusta", &mut buf);
        let s = String::from_utf8(buf).unwrap();
        assert!(s.starts_with("#compdef rusta"), "zsh script missing #compdef header: {s:.80}");
    }
}
