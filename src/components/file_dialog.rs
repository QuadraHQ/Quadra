use iced::Task;
use rfd::AsyncFileDialog;
use std::path::PathBuf;

pub fn pick_file(title: &str, extensions: &[&str]) -> Task<Option<PathBuf>> {
    let mut dialog = AsyncFileDialog::new().set_title(title);
    if !extensions.is_empty() {
        dialog = dialog.add_filter(extensions.join(", "), extensions);
    }
    Task::perform(dialog.pick_file(), |chosen| {
        chosen.map(|file| file.path().to_owned())
    })
}

pub fn pick_folder(title: &str) -> Task<Option<PathBuf>> {
    let dialog = AsyncFileDialog::new().set_title(title);
    Task::perform(dialog.pick_folder(), |chosen| {
        chosen.map(|folder| folder.path().to_owned())
    })
}

pub fn save_file(title: &str, initial_name: &str) -> Task<Option<PathBuf>> {
    let dialog = AsyncFileDialog::new()
        .set_title(title)
        .set_file_name(initial_name);
    Task::perform(dialog.save_file(), |chosen| {
        chosen.map(|file| file.path().to_owned())
    })
}
