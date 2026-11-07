use super::paths;
use crate::state::{AppState, FastFlagValue, FastFlags};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

const VR_LIBRARY: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/libQuadraVR.dylib"));
const VR_LIBRARY_NAME: &str = "libQuadraVR.dylib";

const INSERT_FORCE_MARKER: &[u8] = b"ROSEMARY_DYLD_INSERT_FORCE";
const SCAN_CHUNK_SIZE: usize = 8 << 20;

const RUNTIMES_WITHOUT_ACTIVE_MANIFEST: [&str; 4] = [
    "/usr/share/openxr/1/openxr_wivrn.json",
    "/usr/share/openxr/1/openxr_monado.json",
    "/usr/local/share/openxr/1/openxr_wivrn.json",
    "/usr/local/share/openxr/1/openxr_monado.json",
];
const ACTIVE_RUNTIME_MANIFEST: &str = "openxr/1/active_runtime.json";
const SYSTEM_ACTIVE_RUNTIME_MANIFEST: &str = "/etc/openxr/1/active_runtime.json";
const DEFAULT_XDG_CONFIG_DIRS: &str = "/etc/xdg";
const OPENXR_LOADER: &str = "libopenxr_loader.so.1";
const LIBRARY_DIRECTORIES: [&str; 4] = [
    "/usr/lib",
    "/usr/lib64",
    "/usr/lib/x86_64-linux-gnu",
    "/usr/local/lib",
];
const EGL_VENDOR_DIRECTORIES: [&str; 2] =
    ["/usr/share/glvnd/egl_vendor.d", "/etc/glvnd/egl_vendor.d"];

const OPENGL_RENDERER_FLAGS: [&str; 2] = [
    "FFlagDebugGraphicsPreferOpenGL",
    "FFlagDebugGraphicsDisableMetal",
];

static SCANNED_ROSEMARY_BINARIES: Mutex<BTreeMap<PathBuf, ScannedBinary>> =
    Mutex::new(BTreeMap::new());

struct ScannedBinary {
    modified: SystemTime,
    size: u64,
    supports_vr: bool,
}

pub struct VrSettings {
    mirror: bool,
    render_scale: u32,
    zink: bool,
}

impl VrSettings {
    pub fn wanted_by(state: &AppState) -> Option<Self> {
        state.vr().then(|| Self {
            mirror: state.vr_mirror(),
            render_scale: state.vr_render_scale(),
            zink: state.vr_zink(),
        })
    }
}

pub struct VrSession {
    library: PathBuf,
    runtime: OpenXrRuntime,
    settings: VrSettings,
    mesa_egl_vendor: Option<PathBuf>,
}

pub struct OpenXrRuntime {
    manifest: PathBuf,
    name: String,
}

#[derive(Deserialize)]
struct RuntimeManifest {
    runtime: RuntimeDescription,
}

#[derive(Deserialize)]
struct RuntimeDescription {
    #[serde(default)]
    name: Option<String>,
    library_path: String,
}

impl VrSession {
    pub fn environment(&self) -> Vec<(String, String)> {
        let mode = if self.settings.mirror {
            "openxr"
        } else {
            "openxr-headset"
        };
        let guest_environment = [
            format!("QUADRA_VR={mode}"),
            format!("QUADRA_VR_SCALE={}", self.settings.render_scale),
            format!("QUADRA_XR_RUNTIME_JSON={}", self.runtime.manifest.display()),
        ]
        .join("\n");

        let mut environment = vec![
            (
                "ROSEMARY_DYLD_INSERT_LIBRARIES".to_owned(),
                self.library.display().to_string(),
            ),
            ("ROSEMARY_DYLD_INSERT_FORCE".to_owned(), "1".to_owned()),
            ("ROSEMARY_ENVIRON".to_owned(), guest_environment),
            ("SDL_VIDEO_FORCE_EGL".to_owned(), "1".to_owned()),
        ];
        if let Some(vendor) = &self.mesa_egl_vendor {
            environment.push(("MESA_LOADER_DRIVER_OVERRIDE".to_owned(), "zink".to_owned()));
            environment.push((
                "__EGL_VENDOR_LIBRARY_FILENAMES".to_owned(),
                vendor.display().to_string(),
            ));
        }
        environment
    }

    pub fn runs_on_zink(&self) -> bool {
        self.mesa_egl_vendor.is_some()
    }

    pub fn description(&self) -> String {
        let shown_on = if self.settings.mirror {
            "mirrored in the game window"
        } else {
            "headset only"
        };
        let renderer = if self.runs_on_zink() {
            "; OpenGL through zink (Vulkan)"
        } else {
            ""
        };
        format!(
            "VR: {} ({}); render scale {}%; {shown_on}{renderer}",
            self.runtime.name,
            self.runtime.manifest.display(),
            self.settings.render_scale
        )
    }
}

pub fn with_opengl_renderer(flags: FastFlags) -> FastFlags {
    OPENGL_RENDERER_FLAGS.iter().fold(flags, |flags, name| {
        flags.with(name, Some(FastFlagValue::Toggle(true)))
    })
}

pub fn prepare(settings: VrSettings, rosemary: &Path) -> Result<VrSession, String> {
    let runtime = check_requirements(rosemary)?;
    let mesa_egl_vendor =
        if settings.zink {
            Some(mesa_egl_vendor().ok_or(
                "OpenGL on Vulkan (zink) needs Mesa's EGL, but its vendor file wasn't found.",
            )?)
        } else {
            None
        };
    let library =
        write_library().map_err(|error| format!("Couldn't save the VR library: {error}"))?;

    Ok(VrSession {
        library,
        runtime,
        settings,
        mesa_egl_vendor,
    })
}

pub fn status(rosemary: Option<&Path>) -> String {
    let Some(rosemary) = rosemary else {
        return "Choose an installed Rosemary to use VR.".to_owned();
    };
    match check_requirements(rosemary) {
        Ok(runtime) => format!(
            "Uses {}. Start it and connect the headset before playing.",
            runtime.name
        ),
        Err(reason) => reason,
    }
}

fn check_requirements(rosemary: &Path) -> Result<OpenXrRuntime, String> {
    if VR_LIBRARY.is_empty() {
        return Err(
            "This build of Quadra has no VR library. It's built when clang and lld are installed."
                .to_owned(),
        );
    }
    if !rosemary_supports_vr(rosemary) {
        return Err(
            "The selected Rosemary can't load VR. Choose Quadra's Rosemary under Runtime."
                .to_owned(),
        );
    }
    if !openxr_loader_is_installed() {
        return Err(
            "The OpenXR loader isn't installed (package openxr or libopenxr-loader1).".to_owned(),
        );
    }
    active_runtime()
        .ok_or_else(|| "No OpenXR runtime found. Install and start WiVRn or Monado.".to_owned())
}

fn write_library() -> std::io::Result<PathBuf> {
    let path = paths::vr_directory().join(VR_LIBRARY_NAME);
    if fs::read(&path).ok().as_deref() != Some(VR_LIBRARY) {
        fs::create_dir_all(paths::vr_directory())?;
        let unfinished_path = path.with_extension("partial");
        fs::write(&unfinished_path, VR_LIBRARY)?;
        fs::set_permissions(&unfinished_path, fs::Permissions::from_mode(0o755))?;
        fs::rename(&unfinished_path, &path)?;
    }
    Ok(path)
}

fn rosemary_supports_vr(rosemary: &Path) -> bool {
    let real_runtime = rosemary
        .canonicalize()
        .ok()
        .and_then(|program| Some(program.parent()?.parent()?.join("libexec").join("rosemary")));
    [Some(rosemary.to_owned()), real_runtime]
        .into_iter()
        .flatten()
        .any(|binary| binary_supports_vr(&binary))
}

fn binary_supports_vr(binary: &Path) -> bool {
    let Ok(metadata) = binary.metadata() else {
        return false;
    };
    let Ok(modified) = metadata.modified() else {
        return false;
    };
    let mut scanned = SCANNED_ROSEMARY_BINARIES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    if let Some(earlier_scan) = scanned.get(binary)
        && earlier_scan.modified == modified
        && earlier_scan.size == metadata.len()
    {
        return earlier_scan.supports_vr;
    }

    let supports_vr = contains_marker(binary, INSERT_FORCE_MARKER);
    scanned.insert(
        binary.to_owned(),
        ScannedBinary {
            modified,
            size: metadata.len(),
            supports_vr,
        },
    );
    supports_vr
}

fn contains_marker(path: &Path, marker: &[u8]) -> bool {
    let Ok(mut file) = File::open(path) else {
        return false;
    };
    let mut chunk = vec![0; SCAN_CHUNK_SIZE];
    let mut end_of_previous_chunk = Vec::new();

    loop {
        let read_size = match file.read(&mut chunk) {
            Ok(0) | Err(_) => return false,
            Ok(read_size) => read_size,
        };
        let mut searched = std::mem::take(&mut end_of_previous_chunk);
        searched.extend_from_slice(&chunk[..read_size]);
        if searched
            .windows(marker.len())
            .any(|window| window == marker)
        {
            return true;
        }
        let kept_from = searched.len().saturating_sub(marker.len() - 1);
        end_of_previous_chunk = searched[kept_from..].to_vec();
    }
}

fn openxr_loader_is_installed() -> bool {
    LIBRARY_DIRECTORIES
        .iter()
        .any(|directory| Path::new(directory).join(OPENXR_LOADER).exists())
}

fn active_runtime() -> Option<OpenXrRuntime> {
    runtime_manifest_candidates()
        .into_iter()
        .filter(|candidate| candidate.is_file())
        .find_map(|candidate| usable_runtime(&candidate))
}

fn runtime_manifest_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(chosen) = std::env::var_os("XR_RUNTIME_JSON").filter(|value| !value.is_empty()) {
        candidates.push(PathBuf::from(chosen));
    }
    candidates.push(gtk::glib::user_config_dir().join(ACTIVE_RUNTIME_MANIFEST));
    let config_directories = std::env::var("XDG_CONFIG_DIRS")
        .ok()
        .filter(|directories| !directories.is_empty())
        .unwrap_or_else(|| DEFAULT_XDG_CONFIG_DIRS.to_owned());
    candidates.extend(
        config_directories
            .split(':')
            .filter(|directory| !directory.is_empty())
            .map(|directory| Path::new(directory).join(ACTIVE_RUNTIME_MANIFEST)),
    );
    candidates.push(PathBuf::from(SYSTEM_ACTIVE_RUNTIME_MANIFEST));
    candidates.extend(RUNTIMES_WITHOUT_ACTIVE_MANIFEST.iter().map(PathBuf::from));
    candidates
}

fn usable_runtime(manifest: &Path) -> Option<OpenXrRuntime> {
    let contents = fs::read_to_string(manifest).ok()?;
    let description = serde_json::from_str::<RuntimeManifest>(&contents)
        .ok()?
        .runtime;
    let library_path = Path::new(&description.library_path);
    let manifest = manifest.canonicalize().ok()?;

    let library_exists = if description.library_path.is_empty() {
        false
    } else if !description.library_path.contains('/') {
        true
    } else if library_path.is_absolute() {
        library_path.is_file()
    } else {
        manifest.parent()?.join(library_path).is_file()
    };
    if !library_exists {
        return None;
    }

    let name = description
        .name
        .filter(|name| !name.is_empty())
        .or_else(|| Some(manifest.file_stem()?.to_string_lossy().into_owned()))?;
    Some(OpenXrRuntime { manifest, name })
}

fn mesa_egl_vendor() -> Option<PathBuf> {
    EGL_VENDOR_DIRECTORIES.iter().find_map(|directory| {
        let mut vendors: Vec<PathBuf> = fs::read_dir(directory)
            .ok()?
            .filter_map(|entry| Some(entry.ok()?.path()))
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.contains("mesa") && name.ends_with(".json"))
            })
            .collect();
        vendors.sort();
        vendors.into_iter().next()
    })
}
