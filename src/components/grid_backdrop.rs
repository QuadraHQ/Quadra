use crate::state::{AppState, Theme};
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gdk, glib, graphene};

const CELL_SIZE: usize = 32;
const LINE_WIDTH: f32 = 1.0;
const BACKGROUND: gdk::RGBA = gdk::RGBA::new(16.0 / 255.0, 20.0 / 255.0, 27.0 / 255.0, 1.0);
const LINE: gdk::RGBA = gdk::RGBA::new(128.0 / 255.0, 158.0 / 255.0, 197.0 / 255.0, 0.09);

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct GridBackdrop;

    #[glib::object_subclass]
    impl ObjectSubclass for GridBackdrop {
        const NAME: &'static str = "QuadraGridBackdrop";
        type Type = super::GridBackdrop;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for GridBackdrop {}

    impl WidgetImpl for GridBackdrop {
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let widget = self.obj();
            let (width, height) = (widget.width(), widget.height());
            let (full_width, full_height) = (width as f32, height as f32);

            snapshot.append_color(
                &BACKGROUND,
                &graphene::Rect::new(0.0, 0.0, full_width, full_height),
            );
            for x in (0..width).step_by(CELL_SIZE) {
                snapshot.append_color(
                    &LINE,
                    &graphene::Rect::new(x as f32, 0.0, LINE_WIDTH, full_height),
                );
            }
            for y in (0..height).step_by(CELL_SIZE) {
                snapshot.append_color(
                    &LINE,
                    &graphene::Rect::new(0.0, y as f32, full_width, LINE_WIDTH),
                );
            }
        }
    }
}

glib::wrapper! {
    pub struct GridBackdrop(ObjectSubclass<imp::GridBackdrop>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

pub fn grid_backdrop(state: &AppState) -> GridBackdrop {
    let backdrop: GridBackdrop = glib::Object::builder()
        .property("hexpand", true)
        .property("vexpand", true)
        .property("can-target", false)
        .build();

    state
        .bind_property("theme", &backdrop, "visible")
        .transform_to(|_, theme: Theme| Some(theme == Theme::Grid))
        .sync_create()
        .build();

    backdrop
}
