use adw::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

const FADE_IN_MILLISECONDS: u32 = 180;
const FADE_OUT_MILLISECONDS: u32 = 140;

pub struct Fade {
    widget: gtk::Widget,
    shown: Cell<bool>,
    animation: RefCell<Option<adw::TimedAnimation>>,
}

impl Fade {
    pub fn new(widget: &impl IsA<gtk::Widget>) -> Rc<Self> {
        widget.set_visible(false);
        widget.set_opacity(0.0);
        Rc::new(Self {
            widget: widget.clone().upcast(),
            shown: Cell::new(false),
            animation: RefCell::new(None),
        })
    }

    pub fn is_shown(&self) -> bool {
        self.shown.get()
    }

    pub fn is_visible(&self) -> bool {
        self.widget.is_visible()
    }

    pub fn show(self: &Rc<Self>) {
        if self.shown.replace(true) {
            return;
        }
        self.widget.set_visible(true);
        self.fade_to(1.0, FADE_IN_MILLISECONDS, |_| {});
    }

    pub fn hide(self: &Rc<Self>, when_hidden: impl Fn() + 'static) {
        if !self.shown.replace(false) {
            return;
        }
        self.fade_to(0.0, FADE_OUT_MILLISECONDS, move |fade| {
            if !fade.shown.get() {
                fade.widget.set_visible(false);
            }
            when_hidden();
        });
    }

    pub fn hide_now(&self) {
        if let Some(animation) = self.animation.take() {
            animation.pause();
        }
        self.shown.set(false);
        self.widget.set_opacity(0.0);
        self.widget.set_visible(false);
    }

    fn fade_to(
        self: &Rc<Self>,
        opacity: f64,
        milliseconds: u32,
        when_done: impl Fn(&Self) + 'static,
    ) {
        if let Some(previous) = self.animation.take() {
            previous.pause();
        }

        let widget = self.widget.clone();
        let animation = adw::TimedAnimation::builder()
            .widget(&self.widget)
            .value_from(self.widget.opacity())
            .value_to(opacity)
            .duration(milliseconds)
            .easing(adw::Easing::EaseOutCubic)
            .target(&adw::CallbackAnimationTarget::new(move |value| {
                widget.set_opacity(value)
            }))
            .build();
        let this: Weak<Self> = Rc::downgrade(self);
        animation.connect_done(move |_| {
            if let Some(fade) = this.upgrade() {
                when_done(&fade);
            }
        });
        animation.play();
        self.animation.replace(Some(animation));
    }
}
