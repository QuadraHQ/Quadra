use crate::backend::{self, LaunchRequest, StudioTarget};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const ROBLOX_OPTION: &str = "--roblox";
const STUDIO_OPTION: &str = "--studio";
const HELP_OPTIONS: [&str; 2] = ["--help", "-h"];
const END_OF_OPTIONS: &str = "--";
const STUDIO_LINK_SCHEME_PREFIX: &str = "roblox-studio";
const HARDWARE_REPORT_OPTION: &str = "--hardware-report";
const COLLECT_DIAGNOSTICS_OPTION: &str = "--collect-diagnostics";
const USAGE: &str = "\
Usage:
  quadra [OPTION…] [LINK|PLACE FILE]

Options:
  -h, --help               Show this help
  --roblox                 Start Roblox without opening Quadra, following a roblox:// link if given
  --studio                 Start Roblox Studio without opening Quadra, following a link or opening a place file if given
  --hardware-report        Print the hardware and driver details useful for bug reports, then exit
  --collect-diagnostics    Save a diagnostics archive (in the given folder, or Documents), then exit
";

#[derive(Debug, Clone, Default)]
pub struct Invocation {
    pub arguments: Vec<String>,
    pub directory: PathBuf,
}

impl Invocation {
    pub fn of_this_process() -> Self {
        Self {
            arguments: std::env::args().skip(1).collect(),
            directory: std::env::current_dir().unwrap_or_default(),
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Options {
    roblox: bool,
    studio: bool,
    target: Option<String>,
}

fn parse(arguments: &[String]) -> Result<Options, String> {
    let mut options = Options::default();
    let mut remaining = Vec::new();
    let mut arguments = arguments.iter();

    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            ROBLOX_OPTION => options.roblox = true,
            STUDIO_OPTION => options.studio = true,
            END_OF_OPTIONS => remaining.extend(arguments.by_ref().cloned()),
            option if option.starts_with('-') && option.len() > 1 => {
                return Err(format!("Unknown option {option}"));
            }
            _ => remaining.push(argument.clone()),
        }
    }

    options.target = remaining.into_iter().next();
    Ok(options)
}

pub fn run_report_if_requested(invocation: &Invocation) -> Option<ExitCode> {
    let arguments = &invocation.arguments;
    let is_requested = |option: &str| arguments.iter().position(|argument| argument == option);

    if HELP_OPTIONS
        .iter()
        .any(|option| is_requested(option).is_some())
    {
        print!("{USAGE}");
        return Some(ExitCode::SUCCESS);
    }

    if is_requested(HARDWARE_REPORT_OPTION).is_some() {
        print!("{}", backend::hardware_report());
        return Some(ExitCode::SUCCESS);
    }

    if let Some(position) = is_requested(COLLECT_DIAGNOSTICS_OPTION) {
        let directory = arguments
            .get(position + 1)
            .filter(|argument| !argument.starts_with('-'))
            .map_or_else(backend::diagnostics_directory, PathBuf::from);
        return match backend::collect_diagnostics(&directory) {
            Ok(archive) => {
                println!("{}", archive.display());
                Some(ExitCode::SUCCESS)
            }
            Err(error) => {
                eprintln!("Couldn't collect the diagnostics: {error:#}");
                Some(ExitCode::FAILURE)
            }
        };
    }

    if let Err(problem) = parse(arguments) {
        eprintln!("{problem}\n\n{USAGE}");
        return Some(ExitCode::FAILURE);
    }

    None
}

pub fn launch_requested_by(invocation: &Invocation) -> Option<LaunchRequest> {
    let options = parse(&invocation.arguments).ok()?;
    let directory = &invocation.directory;

    if options.studio {
        return Some(LaunchRequest::Studio(studio_target(
            directory,
            options.target,
        )));
    }
    if options.roblox {
        return Some(LaunchRequest::Roblox {
            link: options.target,
        });
    }

    let target = options.target?;
    let is_roblox_link =
        scheme_of(&target).is_some_and(|scheme| !scheme.starts_with(STUDIO_LINK_SCHEME_PREFIX));
    if is_roblox_link {
        Some(LaunchRequest::Roblox { link: Some(target) })
    } else {
        Some(LaunchRequest::Studio(studio_target(
            directory,
            Some(target),
        )))
    }
}

fn studio_target(directory: &Path, target: Option<String>) -> StudioTarget {
    let Some(target) = target else {
        return StudioTarget::StartPage;
    };
    if scheme_of(&target).is_some() {
        return StudioTarget::Link(target);
    }
    let place_file = target.strip_prefix("file://").unwrap_or(&target);
    StudioTarget::PlaceFile(directory.join(percent_decoded(place_file)))
}

fn percent_decoded(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut position = 0;
    while position < bytes.len() {
        let escaped = (bytes[position] == b'%')
            .then(|| text.get(position + 1..position + 3))
            .flatten()
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        match escaped {
            Some(byte) => {
                decoded.push(byte);
                position += 3;
            }
            None => {
                decoded.push(bytes[position]);
                position += 1;
            }
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn scheme_of(argument: &str) -> Option<String> {
    let (scheme, _) = argument.split_once(':')?;
    let mut characters = scheme.chars();
    let is_scheme = characters.next()?.is_ascii_alphabetic()
        && characters
            .all(|character| character.is_ascii_alphanumeric() || "+-.".contains(character));
    (is_scheme && scheme != "file").then(|| scheme.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn invocation(arguments: &[&str]) -> Invocation {
        Invocation {
            arguments: arguments
                .iter()
                .map(|argument| (*argument).to_owned())
                .collect(),
            directory: PathBuf::from("/home/someone/Games"),
        }
    }

    #[test]
    fn plain_start_requests_nothing() {
        assert_eq!(launch_requested_by(&invocation(&[])), None);
    }

    #[test]
    fn roblox_links_start_roblox() {
        assert_eq!(
            launch_requested_by(&invocation(&["roblox-player:1+launchmode:play"])),
            Some(LaunchRequest::Roblox {
                link: Some("roblox-player:1+launchmode:play".to_owned())
            })
        );
        assert_eq!(
            launch_requested_by(&invocation(&["--roblox"])),
            Some(LaunchRequest::Roblox { link: None })
        );
    }

    #[test]
    fn studio_links_and_place_files_open_studio() {
        assert_eq!(
            launch_requested_by(&invocation(&["roblox-studio:1+task:EditPlace"])),
            Some(LaunchRequest::Studio(StudioTarget::Link(
                "roblox-studio:1+task:EditPlace".to_owned()
            )))
        );
        assert_eq!(
            launch_requested_by(&invocation(&["My%20Place.rbxl"])),
            Some(LaunchRequest::Studio(StudioTarget::PlaceFile(
                PathBuf::from("/home/someone/Games/My Place.rbxl")
            )))
        );
        assert_eq!(
            launch_requested_by(&invocation(&["--studio", "file:///tmp/place.rbxl"])),
            Some(LaunchRequest::Studio(StudioTarget::PlaceFile(
                PathBuf::from("/tmp/place.rbxl")
            )))
        );
    }

    #[test]
    fn unknown_options_are_refused() {
        assert!(parse(&["--nope".to_owned()]).is_err());
    }
}
