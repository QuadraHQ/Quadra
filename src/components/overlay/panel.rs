use super::fade::Fade;
use crate::state::{PanelId, Placement};
use gtk::prelude::*;
use std::cell::Cell;
use std::rc::Rc;

#[derive(Clone, Copy)]
pub enum PanelChange {
    Moved(Placement),
    Settled,
    Pinned(bool),
    Shown(bool),
}

pub type PanelListener = Rc<dyn Fn(PanelId, PanelChange)>;

pub struct Panel {
    pub id: PanelId,
    pub frame: gtk::Box,
    pub fade: Rc<Fade>,
    pin: gtk::ToggleButton,
}

impl Panel {
    pub fn new(id: PanelId, content: &impl IsA<gtk::Widget>, listener: &PanelListener) -> Self {
        let pin = gtk::ToggleButton::builder()
            .icon_name("mark-location-symbolic")
            .tooltip_text("Keep visible while the overlay is closed")
            .css_classes(["flat", "circular"])
            .build();
        pin.connect_toggled({
            let listener = listener.clone();
            move |pin| listener(id, PanelChange::Pinned(pin.is_active()))
        });

        let close = gtk::Button::builder()
            .icon_name("window-close-symbolic")
            .tooltip_text("Close")
            .css_classes(["flat", "circular"])
            .build();
        close.connect_clicked({
            let listener = listener.clone();
            move |_| listener(id, PanelChange::Shown(false))
        });

        let title = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        title.add_css_class("overlay-panel-title");
        title.append(&gtk::Image::from_icon_name(id.icon_name()));
        title.append(
            &gtk::Label::builder()
                .label(id.title())
                .hexpand(true)
                .xalign(0.0)
                .build(),
        );
        title.append(&pin);
        title.append(&close);

        content.set_vexpand(true);

        let frame = gtk::Box::new(gtk::Orientation::Vertical, 0);
        frame.add_css_class("overlay-panel");
        frame.append(&title);
        frame.append(content);
        frame.append(&resize_grip(id, &frame, listener));

        let fade = Fade::new(&frame);
        let panel = Self {
            id,
            frame,
            fade,
            pin,
        };
        panel.raise_when_pressed();
        move_by_dragging(id, &title, &panel.frame, listener);
        panel
    }

    pub fn set_pinned(&self, pinned: bool) {
        self.pin.set_active(pinned);
    }

    fn raise_when_pressed(&self) {
        let press = gtk::GestureClick::new();
        press.set_propagation_phase(gtk::PropagationPhase::Capture);
        press.connect_pressed(|press, _, _, _| {
            let Some(frame) = press.widget() else { return };
            let Some(canvas) = frame.parent() else { return };
            let top = canvas.last_child();
            if top.as_ref() != Some(&frame) {
                frame.insert_after(&canvas, top.as_ref());
            }
        });
        self.frame.add_controller(press);
    }
}

fn placement_of(frame: &gtk::Box) -> Option<Placement> {
    let canvas = frame.parent().and_downcast::<gtk::Fixed>()?;
    let (x, y) = canvas.child_position(frame);
    Some(Placement {
        x: x as i32,
        y: y as i32,
        width: frame.width(),
        height: frame.height(),
    })
}

fn canvas_size_of(frame: &gtk::Box) -> (i32, i32) {
    frame.parent().map_or((i32::MAX, i32::MAX), |canvas| {
        (canvas.width(), canvas.height())
    })
}

fn move_by_dragging(id: PanelId, title: &gtk::Box, frame: &gtk::Box, listener: &PanelListener) {
    let start = Rc::new(Cell::new(None));
    let drag = gtk::GestureDrag::new();

    drag.connect_drag_begin({
        let start = start.clone();
        let frame = frame.clone();
        move |_, _, _| start.set(placement_of(&frame))
    });
    drag.connect_drag_update({
        let listener = listener.clone();
        let frame = frame.clone();
        move |_, offset_x, offset_y| {
            let Some(start) = start.get() else { return };
            let (canvas_width, canvas_height) = canvas_size_of(&frame);
            let moved = Placement {
                x: start.x + offset_x as i32,
                y: start.y + offset_y as i32,
                ..start
            };
            listener(
                id,
                PanelChange::Moved(moved.kept_inside(canvas_width, canvas_height)),
            );
        }
    });
    drag.connect_drag_end({
        let listener = listener.clone();
        move |_, _, _| listener(id, PanelChange::Settled)
    });

    title.add_controller(drag);
}

fn resize_grip(id: PanelId, frame: &gtk::Box, listener: &PanelListener) -> gtk::Widget {
    let grip = gtk::Box::builder()
        .halign(gtk::Align::End)
        .css_classes(["overlay-grip"])
        .cursor(
            &gtk::gdk::Cursor::from_name("se-resize", None).unwrap_or_else(|| {
                gtk::gdk::Cursor::from_name("default", None).expect("the default cursor exists")
            }),
        )
        .build();

    let start = Rc::new(Cell::new(None));
    let drag = gtk::GestureDrag::new();
    drag.connect_drag_begin({
        let start = start.clone();
        let frame = frame.clone();
        move |_, _, _| start.set(placement_of(&frame))
    });
    drag.connect_drag_update({
        let listener = listener.clone();
        let frame = frame.clone();
        move |_, offset_x, offset_y| {
            let Some(start) = start.get() else { return };
            let (canvas_width, canvas_height) = canvas_size_of(&frame);
            let resized = Placement {
                width: start.width + offset_x as i32,
                height: start.height + offset_y as i32,
                ..start
            };
            listener(
                id,
                PanelChange::Moved(resized.kept_inside(canvas_width, canvas_height)),
            );
        }
    });
    drag.connect_drag_end({
        let listener = listener.clone();
        move |_, _, _| listener(id, PanelChange::Settled)
    });
    grip.add_controller(drag);

    grip.upcast()
}
