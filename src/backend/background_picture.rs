use crate::state::{AppState, BackgroundImage};
use anyhow::Context;
use iced::widget::image::Handle;
use image::imageops::FilterType;
use std::path::PathBuf;

const LONGEST_SIDE: u32 = 2560;

pub fn initialize(state: &AppState) {
    state.connect_property_notify("background-picture", load);
    load(state);
}

fn load(state: &AppState) {
    let path = state.background_picture();
    if path.is_empty() {
        state.set_background_image(BackgroundImage(None));
        return;
    }
    let state = state.clone();
    super::background::run_quietly(
        move || decoded(PathBuf::from(path)),
        move |result| match result {
            Ok(handle) => state.set_background_image(BackgroundImage(Some(handle))),
            Err(error) => {
                eprintln!("Couldn't show the background picture: {error:#}");
                state.set_background_image(BackgroundImage(None));
            }
        },
    );
}

fn decoded(path: PathBuf) -> anyhow::Result<Handle> {
    let picture = image::ImageReader::open(&path)
        .with_context(|| format!("Couldn't open {}", path.display()))?
        .with_guessed_format()?
        .decode()
        .with_context(|| format!("Couldn't read {}", path.display()))?;
    let picture = if picture.width().max(picture.height()) > LONGEST_SIDE {
        picture.resize(LONGEST_SIDE, LONGEST_SIDE, FilterType::Triangle)
    } else {
        picture
    };
    let rgba = picture.into_rgba8();
    Ok(Handle::from_rgba(
        rgba.width(),
        rgba.height(),
        rgba.into_raw(),
    ))
}
