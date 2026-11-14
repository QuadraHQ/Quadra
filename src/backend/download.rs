use super::background::Progress;
use super::req_agent::download_agent;
use anyhow::Context;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

const CHUNK_SIZE: usize = 64 * 1024;

pub fn download_file(url: &str, destination: &Path, progress: &Progress) -> anyhow::Result<()> {
    if let Some(directory) = destination.parent() {
        fs::create_dir_all(directory)?;
    }

    let mut response = download_agent()
        .get(url)
        .call()
        .with_context(|| format!("Couldn't download {url}"))?;
    let total_size = response
        .headers()
        .get("content-length")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|&size| size > 0);

    let unfinished_path = destination.with_extension("partial");
    let mut file = File::create(&unfinished_path)?;
    let mut body = response.body_mut().as_reader();
    let mut chunk = vec![0; CHUNK_SIZE];
    let mut downloaded_size: u64 = 0;

    loop {
        let read_size = body
            .read(&mut chunk)
            .with_context(|| format!("The download of {url} was interrupted"))?;
        if read_size == 0 {
            break;
        }
        file.write_all(&chunk[..read_size])?;
        downloaded_size += read_size as u64;
        if let Some(total_size) = total_size {
            progress.fraction(downloaded_size as f64 / total_size as f64);
        }
    }

    file.sync_all()?;
    fs::rename(&unfinished_path, destination)?;
    Ok(())
}

pub fn checksum_listed_for(checksums: &str, file_name: &str) -> Option<String> {
    checksums.lines().find_map(|line| {
        let (checksum, name) = line.split_once(char::is_whitespace)?;
        let name = name.trim_start().trim_start_matches('*');
        (name == file_name).then(|| checksum.to_ascii_lowercase())
    })
}

pub fn sha256_of_file(path: &Path) -> anyhow::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut chunk = vec![0; CHUNK_SIZE];

    loop {
        let read_size = file.read(&mut chunk)?;
        if read_size == 0 {
            break;
        }
        hasher.update(&chunk[..read_size]);
    }

    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

pub fn replace_directory(new_contents: &Path, destination: &Path) -> anyhow::Result<()> {
    if destination.exists() {
        fs::remove_dir_all(destination)
            .with_context(|| format!("Couldn't remove {}", destination.display()))?;
    }
    fs::rename(new_contents, destination)
        .with_context(|| format!("Couldn't move files into {}", destination.display()))?;
    Ok(())
}

pub fn stop_others_from_writing(directory: &Path) -> anyhow::Result<()> {
    const GROUP_AND_OTHERS_WRITE: u32 = 0o022;

    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.is_symlink() {
            continue;
        }
        let mode = metadata.permissions().mode();
        if mode & GROUP_AND_OTHERS_WRITE != 0 {
            fs::set_permissions(
                &path,
                fs::Permissions::from_mode(mode & !GROUP_AND_OTHERS_WRITE),
            )?;
        }
        if metadata.is_dir() {
            stop_others_from_writing(&path)?;
        }
    }
    Ok(())
}

pub fn remove_directory_if_present(directory: &Path) -> anyhow::Result<()> {
    if directory.exists() {
        fs::remove_dir_all(directory)
            .with_context(|| format!("Couldn't remove {}", directory.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::checksum_listed_for;

    #[test]
    fn reads_both_checksum_file_styles() {
        let text = "AAA  plain\nBBB *binary\n";
        assert_eq!(checksum_listed_for(text, "plain").as_deref(), Some("aaa"));
        assert_eq!(checksum_listed_for(text, "binary").as_deref(), Some("bbb"));
        assert_eq!(checksum_listed_for(text, "missing"), None);
    }
}
