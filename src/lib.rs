//! Root binary crate: thin entry only. Game logic lives in `minicraft-client`.

pub fn run_cli(arguments: &[String]) -> Result<(), String> {
    minicraft_client::app::run_cli(arguments)
}
