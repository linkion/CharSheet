//! An MCP server over stdio for reading, editing and playing the characters in the
//! CharSheet library.

mod server;

use dnd_core::Library;
use rmcp::ServiceExt;
use std::process::ExitCode;

const USAGE: &str = "Usage: charsheet-mcp [--db <path to charsheet.db>]";

fn open_library() -> Result<Library, String> {
    let mut args = std::env::args().skip(1);
    let library = match args.next().as_deref() {
        None => Library::open_default(),
        Some("--db") => Library::open(args.next().ok_or("--db needs a path")?),
        Some("-h" | "--help") => {
            println!("{USAGE}");
            std::process::exit(0);
        }
        Some(other) => return Err(format!("unknown argument: {other}\n{USAGE}")),
    };
    library.map_err(|e| format!("Cannot open the character library: {e}"))
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let library = match open_library() {
        Ok(library) => library,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let served = server::CharSheetServer::new(library)
        .serve(rmcp::transport::stdio())
        .await;
    let result = match served {
        Ok(service) => service
            .waiting()
            .await
            .map(|_| ())
            .map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("charsheet-mcp: {e}");
            ExitCode::FAILURE
        }
    }
}
