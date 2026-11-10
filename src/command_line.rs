use crate::backend::{self, LaunchRequest, StudioTarget};
use adw::prelude::*;
use gtk::{gio, glib};

const ROBLOX_OPTION: &str = "roblox";
const STUDIO_OPTION: &str = "studio";
const STUDIO_LINK_SCHEME_PREFIX: &str = "roblox-studio";
const HARDWARE_REPORT_OPTION: &str = "hardware-report";
const COLLECT_DIAGNOSTICS_OPTION: &str = "collect-diagnostics";

pub fn add_options(app: &adw::Application) {
    add_flag(
        app,
        ROBLOX_OPTION,
        "Start Roblox without opening Quadra, following a roblox:// link if given",
    );
    add_flag(
        app,
        STUDIO_OPTION,
        "Start Roblox Studio without opening Quadra, following a link or opening a place file if given",
    );
    add_flag(
        app,
        HARDWARE_REPORT_OPTION,
        "Print the hardware and driver details useful for bug reports, then exit",
    );
    add_flag(
        app,
        COLLECT_DIAGNOSTICS_OPTION,
        "Save a diagnostics archive (in the given folder, or Documents), then exit",
    );
    app.add_main_option(
        glib::OPTION_REMAINING,
        glib::Char(0),
        glib::OptionFlags::NONE,
        glib::OptionArg::StringArray,
        "",
        Some("[LINK|PLACE FILE]"),
    );
}

pub fn run_report_if_requested() -> Option<glib::ExitCode> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let is_requested = |option: &str| {
        arguments
            .iter()
            .position(|argument| *argument == format!("--{option}"))
    };

    if is_requested(HARDWARE_REPORT_OPTION).is_some() {
        print!("{}", backend::hardware_report());
        return Some(glib::ExitCode::SUCCESS);
    }

    let position = is_requested(COLLECT_DIAGNOSTICS_OPTION)?;
    let directory = arguments
        .get(position + 1)
        .filter(|argument| !argument.starts_with('-'))
        .map_or_else(backend::diagnostics_directory, std::path::PathBuf::from);
    match backend::collect_diagnostics(&directory) {
        Ok(archive) => {
            println!("{}", archive.display());
            Some(glib::ExitCode::SUCCESS)
        }
        Err(error) => {
            eprintln!("Couldn't collect the diagnostics: {error:#}");
            Some(glib::ExitCode::FAILURE)
        }
    }
}

pub fn launch_requested_by(command_line: &gio::ApplicationCommandLine) -> Option<LaunchRequest> {
    let options = command_line.options_dict();
    let target = options
        .lookup::<Vec<String>>(glib::OPTION_REMAINING)
        .ok()
        .flatten()
        .and_then(|arguments| arguments.into_iter().next());

    if options.contains(STUDIO_OPTION) {
        return Some(LaunchRequest::Studio(studio_target(command_line, target)));
    }
    if options.contains(ROBLOX_OPTION) {
        return Some(LaunchRequest::Roblox { link: target });
    }

    let target = target?;
    let is_roblox_link =
        scheme_of(&target).is_some_and(|scheme| !scheme.starts_with(STUDIO_LINK_SCHEME_PREFIX));
    if is_roblox_link {
        Some(LaunchRequest::Roblox { link: Some(target) })
    } else {
        Some(LaunchRequest::Studio(studio_target(
            command_line,
            Some(target),
        )))
    }
}

fn add_flag(app: &adw::Application, name: &str, description: &str) {
    app.add_main_option(
        name,
        glib::Char(0),
        glib::OptionFlags::NONE,
        glib::OptionArg::None,
        description,
        None,
    );
}

fn studio_target(
    command_line: &gio::ApplicationCommandLine,
    target: Option<String>,
) -> StudioTarget {
    let Some(target) = target else {
        return StudioTarget::StartPage;
    };
    if scheme_of(&target).is_some() {
        return StudioTarget::Link(target);
    }
    match command_line.create_file_for_arg(&target).path() {
        Some(place_file) => StudioTarget::PlaceFile(place_file),
        None => StudioTarget::StartPage,
    }
}

fn scheme_of(argument: &str) -> Option<String> {
    glib::Uri::peek_scheme(argument)
        .map(|scheme| scheme.to_string())
        .filter(|scheme| scheme != "file")
}
