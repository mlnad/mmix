use std::env;
use std::path::PathBuf;
use std::process::{Command, exit};

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask must be inside workspace")
        .to_path_buf()
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    let file_path = if let Some(path) = args.first() {
        PathBuf::from(path)
    } else {
        project_root().join("examples/hello.mms")
    };

    if !file_path.exists() {
        eprintln!("error: file not found: {}", file_path.display());
        exit(1);
    }

    if file_path.extension().and_then(|e| e.to_str()) != Some("mms") {
        eprintln!("error: expected a .mms file, got: {}", file_path.display());
        exit(1);
    }

    let status = Command::new(env!("CARGO"))
        .args(["run", "--package", "mmixec", "--"])
        .arg(&file_path)
        .status()
        .expect("failed to launch mmixec");

    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            exit(128 + signal);
        }
    }
    exit(status.code().unwrap_or(1));
}
