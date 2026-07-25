mod backup;
mod cli;
mod cxx;
mod document;
mod editor;
mod error;
mod upaths;
mod ui;

use std::path::Path;
use std::process::ExitCode;

use clap::Parser;

use cli::Cli;
use document::Document;
use editor::{Editor, ExitReason};
use error::print_error;
use upaths::expand_path;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let path = expand_path(&cli.path);

    let (initial_text, banner) = match resolve_file(&path, &cli) {
        Ok(result) => result,
        Err(message) => {
            print_error(&message);
            return ExitCode::FAILURE;
        }
    };

    let doc = Document::from_text(&initial_text);
    let editor_result = Editor::new(doc, path, cli.read_only, cli.backup, banner)
        .and_then(Editor::run);

    match editor_result {
        Ok(ExitReason::PermissionDenied) => {
            print_error("Access denied, use sudo");
            ExitCode::FAILURE
        }
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            print_error(&e.to_string());
            ExitCode::FAILURE
        }
    }
}

fn resolve_file(path: &Path, cli: &Cli) -> Result<(String, Option<String>), String> {
    if path.exists() {
        std::fs::read_to_string(path).map(|text| (text, None)).map_err(|e| e.to_string())
    } else if cli.create {
        let parent_missing = path.parent().map(|p| !p.as_os_str().is_empty() && !p.exists()).unwrap_or(false);
        if parent_missing {
            if !cli.parent {
                return Err("No such file or directory".to_string());
            }
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::write(path, "").map_err(|e| e.to_string())?;
            Ok((String::new(), Some("Folders and a new file are created".to_string())))
        } else {
            std::fs::write(path, "").map_err(|e| e.to_string())?;
            Ok((String::new(), Some("New empty file".to_string())))
        }
    } else {
        Err("No such file or directory".to_string())
    }
}
