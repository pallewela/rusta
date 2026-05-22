use std::process::ExitCode;

use clap::Parser;
use rusta_cli::{cli, commands, io, update_check};

fn main() -> ExitCode {
    let args = cli::Cli::parse();

    if let Some(path) = args.log.as_deref() {
        if let Err(e) = io::setup_log_tee(path) {
            eprintln!("rusta: failed to set up --log tee to {}: {}", path, e);
            return ExitCode::from(1);
        }
    }
    io::set_verbose(args.verbose);

    let update_handle = update_check::maybe_spawn();

    let code = match commands::dispatch(args) {
        Ok(code) => code,
        Err(e) => {
            io::err(&format!("{}", e));
            e.exit_code()
        }
    };

    update_check::maybe_finalize(update_handle);

    ExitCode::from(code)
}
