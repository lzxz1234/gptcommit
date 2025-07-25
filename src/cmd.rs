use anyhow::anyhow;
use anyhow::Result;
use std::path::PathBuf;
use which::which;

pub(crate) fn find_executable(name: &str, error_msg: &str) -> Result<PathBuf> {
    let path = which(name).map_err(|_| {
        anyhow!(
            "The `{}` executable could not be found in your PATH. {}",
            name,
            error_msg
        )
    })?;
    println!("Found {} executable at {:?}", name, path);

    Ok(path)
}
