use super::content_files::ContentFiles;
use super::settings::ModSettings;
use crate::state::DeathSound;
use std::io;
use std::path::Path;

const STOCK_OOF: &str = "sounds/oof.ogg";
const DEATH_SOUND: &str = "sounds/ouch.ogg";

const OLD_WALK: &[u8] = include_bytes!("../../../data/mods/sounds/OldWalk.mp3");
const OLD_JUMP: &[u8] = include_bytes!("../../../data/mods/sounds/OldJump.mp3");
const OLD_GET_UP: &[u8] = include_bytes!("../../../data/mods/sounds/OldGetUp.mp3");
const SILENCE: &[u8] = include_bytes!("../../../data/mods/sounds/Empty.mp3");

const OLD_CHARACTER_SOUNDS: [(&str, &[u8]); 7] = [
    ("sounds/action_footsteps_plastic.mp3", OLD_WALK),
    ("sounds/action_jump.mp3", OLD_JUMP),
    ("sounds/action_get_up.mp3", OLD_GET_UP),
    ("sounds/action_falling.mp3", SILENCE),
    ("sounds/action_jump_land.mp3", SILENCE),
    ("sounds/action_swim.mp3", SILENCE),
    ("sounds/impact_water.mp3", SILENCE),
];

pub fn apply_death_sound(files: &mut ContentFiles, settings: &ModSettings) -> io::Result<()> {
    let death_sound = Path::new(DEATH_SOUND);
    match settings.death_sound {
        DeathSound::Stock => Ok(()),
        DeathSound::ClassicOof => {
            let stock_oof = Path::new(STOCK_OOF);
            if !files.exists(stock_oof) || !files.exists(death_sound) {
                return Ok(());
            }
            let oof = files.read(stock_oof)?;
            files.place_bytes(death_sound, &oof)
        }
        DeathSound::Custom if settings.custom_death_sound.is_file() => {
            files.place_file(death_sound, &settings.custom_death_sound)
        }
        DeathSound::Custom => Ok(()),
    }
}

pub fn apply_old_character_sounds(
    files: &mut ContentFiles,
    settings: &ModSettings,
) -> io::Result<()> {
    if !settings.old_character_sounds {
        return Ok(());
    }
    for (relative, sound) in OLD_CHARACTER_SOUNDS {
        files.place_bytes(Path::new(relative), sound)?;
    }
    Ok(())
}
