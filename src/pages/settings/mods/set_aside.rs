use crate::backend;
use adw::prelude::*;
use gtk::glib;
use std::cell::RefCell;
use std::rc::Rc;

type Rows = Rc<RefCell<Vec<adw::ActionRow>>>;

pub fn set_aside_group() -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::builder()
        .title("Set aside by the crash guard")
        .description("Roblox crashed soon after these were added, so they are no longer applied.")
        .build();

    let rows = Rows::default();
    group.connect_map(move |group| show_units(group, &rows));

    group
}

fn show_units(group: &adw::PreferencesGroup, rows: &Rows) {
    for row in rows.borrow_mut().drain(..) {
        group.remove(&row);
    }

    let units = backend::disabled_mods();
    group.set_visible(!units.is_empty());
    for unit in units {
        let row = unit_row(&unit, group, rows);
        group.add(&row);
        rows.borrow_mut().push(row);
    }
}

fn unit_row(unit: &str, group: &adw::PreferencesGroup, rows: &Rows) -> adw::ActionRow {
    let enable_button = gtk::Button::builder()
        .label("Re-enable")
        .valign(gtk::Align::Center)
        .build();
    let unit_name = unit.to_owned();
    enable_button.connect_clicked(glib::clone!(
        #[weak]
        group,
        #[strong]
        rows,
        move |button| match backend::enable_mod(&unit_name) {
            Ok(()) => show_units(&group, &rows),
            Err(error) => {
                button.set_tooltip_text(Some(&format!("Couldn't re-enable {unit_name}: {error}")))
            }
        }
    ));

    let row = adw::ActionRow::builder().title(unit).build();
    row.add_suffix(&enable_button);
    row
}
