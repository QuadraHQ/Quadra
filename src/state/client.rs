use gtk::glib;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "QuadraClient")]
pub enum Client {
    #[default]
    Player,
    Studio,
}

impl Client {
    pub const ALL: [Self; 2] = [Self::Player, Self::Studio];

    pub fn name(self) -> &'static str {
        match self {
            Self::Player => "Roblox",
            Self::Studio => "Roblox Studio",
        }
    }

    pub fn short_name(self) -> &'static str {
        match self {
            Self::Player => "Roblox",
            Self::Studio => "Studio",
        }
    }

    pub fn page_name(self) -> &'static str {
        match self {
            Self::Player => "roblox",
            Self::Studio => "studio",
        }
    }

    pub fn with_page_name(page_name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|client| client.page_name() == page_name)
    }

    pub fn home_title(self) -> &'static str {
        match self {
            Self::Player => "Quadra",
            Self::Studio => "Quadra Studio",
        }
    }
}
