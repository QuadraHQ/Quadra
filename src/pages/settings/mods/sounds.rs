use crate::components::file_row::{Pick, file_row};
use crate::pages::settings::bound_rows::{choice_row, show_when, switch_row};
use crate::state::{AppState, DeathSound};
use adw::prelude::*;

const AUDIO_EXTENSIONS: [&str; 3] = ["ogg", "mp3", "wav"];

pub fn sounds_group(state: &AppState) -> adw::PreferencesGroup {
    let death_sound_row = choice_row::<DeathSound>(state, "death-sound", "Death sound");
    death_sound_row.set_subtitle("The sound played when your character resets or dies");

    let custom_death_sound_row = file_row(
        state,
        "custom-death-sound",
        "Custom death sound",
        &[Pick::File(&AUDIO_EXTENSIONS)],
    );
    show_when(
        &custom_death_sound_row,
        state,
        "death-sound",
        |sound: DeathSound| sound == DeathSound::Custom,
    );

    let group = adw::PreferencesGroup::builder().title("Sounds").build();
    group.add(&death_sound_row);
    group.add(&custom_death_sound_row);
    group.add(&switch_row(
        state,
        "old-character-sounds",
        "Classic movement sounds",
    ));
    group
}
