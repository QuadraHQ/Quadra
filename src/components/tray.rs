use super::icon::app_icon_texture;
use super::play_action::{PROPERTIES_THAT_DECIDE_PLAY_ACTION, PlayAction};
use super::roblox_status::{PROPERTIES_THAT_DECIDE_STATUS, status_text};
use crate::state::AppState;
use crate::utils::is_running_in_flatpak;
use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;
use ksni::blocking::{Handle, TrayMethods};
use ksni::menu::{MenuItem, StandardItem};

const BYTES_PER_ARGB_PIXEL: usize = 4;

#[derive(Clone, Copy)]
pub enum TrayRequest {
    OpenHome,
    OpenSettings,
    ToggleOverlay,
    PerformPlayAction,
    Quit,
}

struct TrayStatus {
    play_action: PlayAction,
    play_action_is_available: bool,
    description: String,
}

impl TrayStatus {
    fn of(state: &AppState) -> Self {
        let client = state.current_client();
        let play_action = PlayAction::for_client(state, client);
        Self {
            play_action,
            play_action_is_available: play_action.is_available(state),
            description: status_text(state, client),
        }
    }
}

struct QuadraTray {
    status: TrayStatus,
    icon: Vec<ksni::Icon>,
    requests: async_channel::Sender<TrayRequest>,
}

impl QuadraTray {
    fn request(&self, request: TrayRequest) {
        self.requests.try_send(request).ok();
    }

    fn menu_item(label: &str, request: TrayRequest) -> MenuItem<Self> {
        StandardItem {
            label: label.to_owned(),
            activate: Box::new(move |tray: &mut Self| tray.request(request)),
            ..Default::default()
        }
        .into()
    }
}

impl ksni::Tray for QuadraTray {
    fn id(&self) -> String {
        "quadra".to_owned()
    }

    fn title(&self) -> String {
        "Quadra".to_owned()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        self.icon.clone()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: "Quadra".to_owned(),
            description: self.status.description.clone(),
            ..Default::default()
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        self.request(TrayRequest::OpenHome);
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let play_action = StandardItem {
            label: self.status.play_action.label().to_owned(),
            enabled: self.status.play_action_is_available,
            activate: Box::new(|tray: &mut Self| tray.request(TrayRequest::PerformPlayAction)),
            ..Default::default()
        };

        vec![
            play_action.into(),
            MenuItem::Separator,
            Self::menu_item("Open Quadra", TrayRequest::OpenHome),
            Self::menu_item("Settings", TrayRequest::OpenSettings),
            Self::menu_item("Toggle overlay", TrayRequest::ToggleOverlay),
            MenuItem::Separator,
            Self::menu_item("Quit", TrayRequest::Quit),
        ]
    }
}

pub struct TrayIcon {
    handle: Handle<QuadraTray>,
    state: AppState,
    watchers: Vec<glib::SignalHandlerId>,
}

impl TrayIcon {
    pub fn show(
        state: &AppState,
        on_request: impl Fn(TrayRequest) + 'static,
    ) -> Result<Self, ksni::Error> {
        let (requests, received_requests) = async_channel::unbounded();
        let tray = QuadraTray {
            status: TrayStatus::of(state),
            icon: app_icon_pixmap(),
            requests,
        };
        let handle = tray.disable_dbus_name(is_running_in_flatpak()).spawn()?;

        glib::spawn_future_local(async move {
            while let Ok(request) = received_requests.recv().await {
                on_request(request);
            }
        });

        Ok(Self {
            watchers: keep_status_shown(state, handle.clone()),
            handle,
            state: state.clone(),
        })
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        for watcher in self.watchers.drain(..) {
            self.state.disconnect(watcher);
        }
        self.handle.shutdown();
    }
}

fn keep_status_shown(state: &AppState, handle: Handle<QuadraTray>) -> Vec<glib::SignalHandlerId> {
    let (status_changes, received_status_changes) = async_channel::unbounded();

    std::thread::spawn(move || {
        while let Ok(status) = received_status_changes.recv_blocking() {
            handle.update(|tray| tray.status = status);
        }
    });

    PROPERTIES_THAT_DECIDE_PLAY_ACTION
        .into_iter()
        .chain(PROPERTIES_THAT_DECIDE_STATUS)
        .map(|property| {
            let status_changes = status_changes.clone();
            state.connect_notify_local(Some(property), move |state, _| {
                status_changes.try_send(TrayStatus::of(state)).ok();
            })
        })
        .collect()
}

fn app_icon_pixmap() -> Vec<ksni::Icon> {
    let texture = match app_icon_texture() {
        Ok(texture) => texture,
        Err(error) => {
            eprintln!("Failed to load the tray icon: {error}");
            return Vec::new();
        }
    };

    let mut downloader = gdk::TextureDownloader::new(&texture);
    downloader.set_format(gdk::MemoryFormat::A8r8g8b8);
    let (pixels, stride) = downloader.download_bytes();
    let row_length = texture.width() as usize * BYTES_PER_ARGB_PIXEL;
    let data = pixels
        .chunks(stride)
        .flat_map(|row| &row[..row_length])
        .copied()
        .collect();

    vec![ksni::Icon {
        width: texture.width(),
        height: texture.height(),
        data,
    }]
}
