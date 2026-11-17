use crate::components::file_row::{ChosenPath, Pick, file_row};
use crate::components::preferences::preference_group;
use crate::components::style::Look;
use crate::pages::settings::bound_rows::{choice_row, switch_row};
use crate::quadra::Message;
use crate::state::{AppState, DeathSound};
use iced::Element;

const AUDIO_FILES: &[Pick] = &[Pick::File(&["ogg", "mp3", "wav"])];

pub fn sounds_group<'a>(state: &AppState, look: Look) -> Element<'a, Message> {
    let death_sound = state.death_sound();
    let custom_death_sound_row = (death_sound == DeathSound::Custom).then(|| {
        file_row(
            state,
            look,
            ChosenPath::DeathSound,
            "Custom death sound",
            AUDIO_FILES,
        )
    });

    preference_group(look)
        .title("Sounds")
        .push(
            choice_row(look, "Death sound", death_sound, |state, sound| {
                state.set_death_sound(sound)
            })
            .subtitle("The sound played when your character resets or dies"),
        )
        .push_maybe(custom_death_sound_row)
        .push(switch_row(
            look,
            "Classic movement sounds",
            state.old_character_sounds(),
            |state, on| state.set_old_character_sounds(on),
        ))
        .into()
}
