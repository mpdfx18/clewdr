use clap::Parser;
use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
    sync::LazyLock,
};
use tracing::error;
use walkdir::WalkDir;

use crate::error::ClewdrError;

pub mod text;

pub static ARG_COOKIE_FILE: LazyLock<Option<PathBuf>> = LazyLock::new(|| {
    let args = crate::Args::parse();
    if let Some(cookie_file) = args.file {
        // canonicalize the path
        cookie_file.canonicalize().ok()
    } else {
        None
    }
});

pub static CLEWDR_DIR: LazyLock<PathBuf> =
    LazyLock::new(|| set_clewdr_dir().expect("Failed to get dir"));
pub const LOG_DIR: &str = "log";

/// Gets and sets up the configuration directory for the application
///
/// In debug mode, uses the current working directory
/// In release mode, uses the directory of the executable
/// Also creates the log directory if it doesn't exist
///
/// # Returns
/// * `Result<PathBuf, ClewdrError>` - The path to the configuration directory on success, or an error
fn set_clewdr_dir() -> Result<PathBuf, ClewdrError> {
    let dir = {
        #[cfg(debug_assertions)]
        {
            // In debug mode, use the current working directory
            // to find the config file
            std::env::current_dir()?
        }
        #[cfg(not(debug_assertions))]
        {
            // In release mode, use the directory of the executable
            // to find the config file
            let exec_path = std::env::current_exe()?;
            let exec_dir = exec_path
                .parent()
                .ok_or_else(|| ClewdrError::PathNotFound("exec dir".to_string()))?
                .canonicalize()?
                .to_path_buf();
            // cd to the exec dir
            std::env::set_current_dir(&exec_dir)?;
            exec_dir
        }
    };
    // create log dir
    #[cfg(feature = "no_fs")]
    {
        return Ok(dir);
    }

    let log_dir = dir.join(LOG_DIR);
    if !log_dir.exists() {
        fs::create_dir_all(&log_dir)?;
    }
    Ok(dir)
}

/// Recursively copies all files and subdirectories from `src` to `dst`
///
/// # Arguments
/// * `src` - Source directory path
/// * `dst` - Destination directory path
///
/// # Returns
/// * `Result<(), ClewdrError>` - Success or an error with details
pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<(), ClewdrError> {
    #[cfg(feature = "no_fs")]
    {
        return Ok(());
    }
    let src = src.as_ref();
    let dst = dst.as_ref();

    if !src.exists() {
        return Err(ClewdrError::PathNotFound(format!(
            "Source directory not found: {}",
            src.display()
        )));
    }

    fs::create_dir_all(dst)?;

    for entry in WalkDir::new(src).min_depth(1) {
        let entry = entry?;

        let path = entry.path();
        let relative_path = path.strip_prefix(src).map_err(|_| {
            ClewdrError::PathNotFound(format!(
                "Failed to strip prefix from path: {}",
                path.display()
            ))
        })?;
        let target_path = dst.join(relative_path);

        if path.is_dir() {
            fs::create_dir_all(&target_path)?;
        } else {
            if let Some(parent) = target_path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }
            fs::copy(path, &target_path)?;
        }
    }

    Ok(())
}

/// Helper function to print out JSON to a file in the log directory
///
/// # Arguments
/// * `json` - The JSON object to serialize and output
/// * `file_name` - The name of the file to write in the log directory
pub fn print_out_json(json: &impl serde::ser::Serialize, file_name: &str) {
    #[cfg(feature = "no_fs")]
    {
        return;
    }
    let text = serde_json::to_string_pretty(json).unwrap_or_default();
    print_out_text(&text, file_name);
}

/// Helper function to print out text to a file in the log directory
///
/// # Arguments
/// * `text` - The text content to write
/// * `file_name` - The name of the file to write in the log directory
pub fn print_out_text(text: &str, file_name: &str) {
    #[cfg(feature = "no_fs")]
    {
        return;
    }
    let Ok(log_dir) = PathBuf::from_str(LOG_DIR);
    let file_name = log_dir.join(file_name);
    let Ok(mut file) = std::fs::File::options()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&file_name)
    else {
        error!("Failed to open file: {}", file_name.display());
        return;
    };
    if let Err(e) = std::io::Write::write_all(&mut file, text.as_bytes()) {
        error!("Failed to write to file: {}\n", e);
    }
}

/// Timezone for the API
pub const TIME_ZONE: &str = "America/New_York";
