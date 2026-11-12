use super::download::sha256_of_file;
use super::roblox;
use crate::state::Client;
use anyhow::{Context, ensure};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

const EXECUTABLE_INSIDE_APP: &str = "Contents/MacOS/RobloxPlayer";
const UNFINISHED_NAME: &str = ".RobloxPlayer.transport-patch";
const RETURN_FALSE: [u8; 4] = [0x31, 0xc0, 0xc3, 0x90];
const FUNCTION_PROLOGUE: [u8; 4] = [0x55, 0x48, 0x89, 0xe5];

struct KnownRelease {
    version: &'static str,
    size: u64,
    untouched_sha256: &'static str,
    transport_selection_offset: usize,
}

const KNOWN_RELEASES: [KnownRelease; 1] = [KnownRelease {
    version: "0.747.0.7471481",
    size: 124_413_696,
    untouched_sha256: "ee09f6cda5362ad9829396ab4e57cb5382401ed1532c2270e01cbb18ab7719dc",
    transport_selection_offset: 0x4C1_1EDB,
}];

pub fn prepare(force_raknet: bool) {
    let executable = roblox::app_path(Client::Player).join(EXECUTABLE_INSIDE_APP);
    match set_forced(&executable, force_raknet) {
        Ok(Outcome::Changed) if force_raknet => eprintln!("Force RakNet: patched Roblox"),
        Ok(Outcome::Changed) => eprintln!("Force RakNet: restored the untouched Roblox"),
        Ok(Outcome::AlreadySet) => {}
        Ok(Outcome::UnknownRelease) if force_raknet => eprintln!(
            "Force RakNet: Quadra doesn't know this Roblox version, so it was left untouched"
        ),
        Ok(Outcome::UnknownRelease) => {}
        Err(error) => eprintln!("Force RakNet: couldn't change Roblox: {error:#}"),
    }
}

#[derive(Debug, PartialEq)]
enum Outcome {
    Changed,
    AlreadySet,
    UnknownRelease,
}

fn set_forced(executable: &Path, forced: bool) -> anyhow::Result<Outcome> {
    let size = fs::metadata(executable)?.len();
    let Some(release) = KNOWN_RELEASES.iter().find(|release| release.size == size) else {
        return Ok(Outcome::UnknownRelease);
    };
    let mut contents = fs::read(executable)?;
    let site = release.transport_selection_offset..release.transport_selection_offset + 4;
    let wanted = if forced {
        RETURN_FALSE
    } else {
        FUNCTION_PROLOGUE
    };
    if contents[site.clone()] == wanted {
        return Ok(Outcome::AlreadySet);
    }
    if contents[site.clone()] != RETURN_FALSE && contents[site.clone()] != FUNCTION_PROLOGUE {
        return Ok(Outcome::UnknownRelease);
    }

    let mut untouched = contents.clone();
    untouched[site.clone()].copy_from_slice(&FUNCTION_PROLOGUE);
    if sha256_of(&untouched) != release.untouched_sha256 {
        return Ok(Outcome::UnknownRelease);
    }

    contents[site].copy_from_slice(&wanted);
    replace_contents(executable, &contents)
        .with_context(|| format!("Couldn't write Roblox {}", release.version))?;
    Ok(Outcome::Changed)
}

fn replace_contents(executable: &Path, contents: &[u8]) -> anyhow::Result<()> {
    let unfinished = executable.with_file_name(UNFINISHED_NAME);
    let mut file = File::create(&unfinished)?;
    file.write_all(contents)?;
    file.set_permissions(fs::metadata(executable)?.permissions())?;
    file.sync_all()?;
    fs::rename(&unfinished, executable)?;
    ensure!(
        sha256_of_file(executable)? == sha256_of(contents),
        "The file changed while it was being written"
    );
    Ok(())
}

fn sha256_of(contents: &[u8]) -> String {
    Sha256::digest(contents)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Outcome, set_forced};
    use std::fs;

    #[test]
    fn leaves_an_unknown_executable_untouched() {
        let directory =
            std::env::temp_dir().join(format!("quadra-transport-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let executable = directory.join("RobloxPlayer");
        fs::write(&executable, b"not a known release").unwrap();

        assert_eq!(
            set_forced(&executable, true).unwrap(),
            Outcome::UnknownRelease
        );
        assert_eq!(fs::read(&executable).unwrap(), b"not a known release");

        fs::remove_dir_all(&directory).unwrap();
    }
}
