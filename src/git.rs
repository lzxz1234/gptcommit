use std::{
    fs::{self},
    path::PathBuf,
    process::Command,
};

use git2::{DiffOptions, Repository};
#[cfg(unix)]
use std::{fs::Permissions, os::unix::prelude::PermissionsExt};

use anyhow::{bail, Result};

pub(crate) fn get_diffs() -> Result<String> {
    let repo = Repository::open_from_env()?;
    let head = repo.head()?.peel_to_tree()?;
    let mut opts = DiffOptions::new();
    opts.ignore_whitespace(true);
    opts.minimal(true);
    let diff = repo.diff_tree_to_index(Some(&head), None, Some(&mut opts))?;
    let mut patches = String::new();
    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        let sign = match line.origin() {
            'H' | 'F' | 'B' => ' ',
            _ => line.origin(),
        };
        patches.push(sign);
        patches.push_str(
            std::str::from_utf8(line.content()).unwrap(),
        );
        true
    })?;

    Ok(patches)
}

/// Given current working directory, return path to .git/hooks
pub(crate) fn get_hooks_path() -> Result<PathBuf> {
    let command_output = Command::new("git")
        .args(["rev-parse", "--show-toplevel", "--git-path", "hooks"])
        .output()?;
    info!("Repo path from git: {:?}", command_output);

    if !command_output.status.success() {
        let stderr = String::from_utf8_lossy(&command_output.stderr);
        bail!("{}", stderr);
    }

    let stdout = String::from_utf8(command_output.stdout).expect("Invalid UTF-8");
    let rel_hooks_path = stdout.lines().last().unwrap().to_string();
    // if hooks dir doesn't exist, create it
    if !std::path::Path::new(&rel_hooks_path).exists() {
        info!("Creating dir at {}", rel_hooks_path);
        // create dirs first otherwise canonicalize will fail
        fs::create_dir_all(&rel_hooks_path)?;
        #[cfg(unix)]
        fs::set_permissions(&rel_hooks_path, Permissions::from_mode(0o700))?;
    }
    // turn relative path into absolute path
    let hooks_path = std::fs::canonicalize(rel_hooks_path)?;
    Ok(hooks_path)
}
