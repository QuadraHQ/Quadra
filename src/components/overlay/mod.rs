mod clock;
mod current_server;
mod dock;
mod fade;
mod panel;
mod panels;
mod surface;

use crate::backend;
use crate::components::shortcut_recorder;
use crate::state::{AppState, Client, OverlayLayout, PanelId};
use dock::Dock;
use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;
use panel::{Panel, PanelChange, PanelListener};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

pub use surface::is_supported;

const PROPERTIES_THAT_DECIDE_AVAILABILITY: [&str; 4] = [
    "overlay-enabled",
    "running",
    "running-client",
    "roblox-graphics-started",
];

pub struct Overlay {
    this: Weak<Self>,
    state: AppState,
    window: gtk::Window,
    canvas: gtk::Fixed,
    dock: Dock,
    panels: Vec<Panel>,
    layout: RefCell<OverlayLayout>,
    opened: Cell<bool>,
    accepted_areas: RefCell<Option<Vec<gdk::Rectangle>>>,
}

impl Overlay {
    pub fn new(state: &AppState) -> Option<Rc<Self>> {
        if !is_supported() {
            return None;
        }
        surface::load_style();

        let overlay = Rc::new_cyclic(|this: &Weak<Self>| {
            let listener: PanelListener = Rc::new({
                let this = this.clone();
                move |id, change| {
                    if let Some(overlay) = this.upgrade() {
                        overlay.panel_changed(id, change);
                    }
                }
            });
            let close_overlay: Rc<dyn Fn()> = Rc::new({
                let this = this.clone();
                move || {
                    if let Some(overlay) = this.upgrade() {
                        overlay.close();
                    }
                }
            });

            let layout = backend::read_overlay_layout();
            let canvas = gtk::Fixed::new();
            let panels: Vec<Panel> = PanelId::ALL
                .into_iter()
                .map(|id| {
                    let panel = Panel::new(id, &panels::content_of(id, state), &listener);
                    let placement = layout.panel(id).placement;
                    canvas.put(&panel.frame, f64::from(placement.x), f64::from(placement.y));
                    panel
                })
                .collect();
            let dock = Dock::new(state, &listener, close_overlay);

            let surface = gtk::Overlay::new();
            surface.set_child(Some(&canvas));
            surface.add_overlay(&dock.widget);
            let window = surface::window();
            window.set_child(Some(&surface));

            Self {
                this: this.clone(),
                state: state.clone(),
                window,
                canvas,
                dock,
                panels,
                layout: RefCell::new(layout),
                opened: Cell::new(false),
                accepted_areas: RefCell::new(None),
            }
        });

        overlay.follow_state();
        overlay.handle_keys();
        overlay.accept_input_only_where_shown();
        overlay.refresh();

        Some(overlay)
    }

    pub fn toggle(&self) {
        if !self.is_available() {
            return;
        }
        self.opened.set(!self.opened.get());
        self.refresh();
    }

    fn close(&self) {
        self.opened.set(false);
        self.refresh();
    }

    fn is_available(&self) -> bool {
        self.state.overlay_enabled()
            && self.state.is_running(Client::Player)
            && self.state.roblox_graphics_started()
    }

    fn follow_state(self: &Rc<Self>) {
        for property in PROPERTIES_THAT_DECIDE_AVAILABILITY {
            self.state.connect_notify_local(
                Some(property),
                glib::clone!(
                    #[weak(rename_to = overlay)]
                    self,
                    move |_, _| overlay.refresh()
                ),
            );
        }
    }

    fn handle_keys(self: &Rc<Self>) {
        let keys = gtk::EventControllerKey::new();
        keys.connect_key_pressed(glib::clone!(
            #[weak(rename_to = overlay)]
            self,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_, key, _, modifiers| {
                let is_shortcut = shortcut_recorder::is_pressed(
                    &overlay.state.overlay_shortcut(),
                    key,
                    modifiers,
                );
                if key == gdk::Key::Escape || is_shortcut {
                    overlay.close();
                    return glib::Propagation::Stop;
                }
                glib::Propagation::Proceed
            }
        ));
        self.window.add_controller(keys);
    }

    fn accept_input_only_where_shown(self: &Rc<Self>) {
        self.window.add_tick_callback(glib::clone!(
            #[weak(rename_to = overlay)]
            self,
            #[upgrade_or]
            glib::ControlFlow::Break,
            move |_, _| {
                overlay.sync_input_areas();
                glib::ControlFlow::Continue
            }
        ));
        self.window.connect_unmap(glib::clone!(
            #[weak(rename_to = overlay)]
            self,
            move |_| {
                overlay.accepted_areas.replace(None);
            }
        ));
    }

    fn sync_input_areas(&self) {
        let areas: Vec<gdk::Rectangle> = if self.opened.get() {
            self.panels
                .iter()
                .map(|panel| (&panel.fade, panel.frame.upcast_ref::<gtk::Widget>()))
                .chain([(
                    &self.dock.fade,
                    self.dock.widget.upcast_ref::<gtk::Widget>(),
                )])
                .filter(|(fade, _)| fade.is_shown())
                .filter_map(|(_, widget)| surface::area_of(widget, &self.window))
                .collect()
        } else {
            Vec::new()
        };

        if self.accepted_areas.borrow().as_ref() == Some(&areas) {
            return;
        }
        surface::accept_input_only_inside(&self.window, &areas);
        self.accepted_areas.replace(Some(areas));
    }

    fn panel_changed(&self, id: PanelId, change: PanelChange) {
        match change {
            PanelChange::Moved(placement) => {
                self.layout
                    .borrow_mut()
                    .update(id, |panel| panel.placement = placement);
                self.place(id);
            }
            PanelChange::Settled => self.save_layout(),
            PanelChange::Pinned(pinned) => {
                if self.layout.borrow().panel(id).pinned != pinned {
                    self.layout
                        .borrow_mut()
                        .update(id, |panel| panel.pinned = pinned);
                    self.save_layout();
                    self.refresh();
                }
            }
            PanelChange::Shown(visible) => {
                if self.layout.borrow().panel(id).visible != visible {
                    self.layout.borrow_mut().update(id, |panel| {
                        panel.visible = visible;
                        panel.pinned &= visible;
                    });
                    self.save_layout();
                    self.refresh();
                }
            }
        }
    }

    fn save_layout(&self) {
        backend::write_overlay_layout(&self.layout.borrow());
    }

    fn place(&self, id: PanelId) {
        let Some(panel) = self.panels.iter().find(|panel| panel.id == id) else {
            return;
        };
        let mut placement = self.layout.borrow().panel(id).placement;
        if self.canvas.width() > 0 {
            placement = placement.kept_inside(self.canvas.width(), self.canvas.height());
        }
        self.canvas
            .move_(&panel.frame, f64::from(placement.x), f64::from(placement.y));
        panel
            .frame
            .set_size_request(placement.width, placement.height);
    }

    fn refresh(&self) {
        if !self.is_available() {
            self.opened.set(false);
            self.hide_everything_now();
            return;
        }
        let layout = self.layout.borrow().clone();
        let opened = self.opened.get();
        let shows_something = opened || layout.has_pinned_panels();

        surface::accept_keyboard(&self.window, opened);
        if shows_something {
            self.window.set_visible(true);
        }

        for panel in &self.panels {
            let panel_layout = layout.panel(panel.id);
            panel.set_pinned(panel_layout.pinned);
            self.dock.set_shown(panel.id, panel_layout.visible);
            self.place(panel.id);
            if panel_layout.visible && (opened || panel_layout.pinned) {
                panel.fade.show();
            } else {
                panel.fade.hide(self.hide_window_when_faded_callback());
            }
        }
        if opened {
            self.dock.fade.show();
        } else {
            self.dock.fade.hide(self.hide_window_when_faded_callback());
        }

        if !shows_something {
            self.hide_window_when_faded();
        }
    }

    fn hide_everything_now(&self) {
        for panel in &self.panels {
            panel.fade.hide_now();
        }
        self.dock.fade.hide_now();
        surface::accept_keyboard(&self.window, false);
        self.window.set_visible(false);
    }

    fn hide_window_when_faded_callback(&self) -> impl Fn() + 'static {
        let this = self.this.clone();
        move || {
            if let Some(overlay) = this.upgrade() {
                overlay.hide_window_when_faded();
            }
        }
    }

    fn hide_window_when_faded(&self) {
        let wanted = self.opened.get() || self.layout.borrow().has_pinned_panels();
        let still_fading =
            self.dock.fade.is_visible() || self.panels.iter().any(|panel| panel.fade.is_visible());
        if !wanted && !still_fading {
            self.window.set_visible(false);
        }
    }
}
