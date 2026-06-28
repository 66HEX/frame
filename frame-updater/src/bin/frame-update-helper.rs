use std::process::ExitCode;

fn main() -> ExitCode {
    match frame_updater::helper::run_from_env_args() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
