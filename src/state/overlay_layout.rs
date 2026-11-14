use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const MINIMUM_PANEL_WIDTH: i32 = 280;
const MINIMUM_PANEL_HEIGHT: i32 = 160;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelId {
    Server,
    History,
    Notes,
    Settings,
}

impl PanelId {
    pub const ALL: [Self; 4] = [Self::Server, Self::History, Self::Notes, Self::Settings];

    pub fn title(self) -> &'static str {
        match self {
            Self::Server => "Server",
            Self::History => "Game history",
            Self::Notes => "Notes",
            Self::Settings => "Settings",
        }
    }

    pub fn icon_name(self) -> &'static str {
        match self {
            Self::Server => "network-server-symbolic",
            Self::History => "document-open-recent-symbolic",
            Self::Notes => "document-edit-symbolic",
            Self::Settings => "emblem-system-symbolic",
        }
    }

    fn default_placement(self) -> Placement {
        let (x, y, width, height) = match self {
            Self::Server => (48, 64, 420, 340),
            Self::History => (500, 64, 420, 380),
            Self::Notes => (48, 436, 420, 300),
            Self::Settings => (952, 64, 400, 360),
        };
        Placement {
            x,
            y,
            width,
            height,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Placement {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Placement {
    pub fn kept_inside(self, canvas_width: i32, canvas_height: i32) -> Self {
        let width = self
            .width
            .clamp(MINIMUM_PANEL_WIDTH, canvas_width.max(MINIMUM_PANEL_WIDTH));
        let height = self.height.clamp(
            MINIMUM_PANEL_HEIGHT,
            canvas_height.max(MINIMUM_PANEL_HEIGHT),
        );
        Self {
            x: self.x.clamp(0, (canvas_width - width).max(0)),
            y: self.y.clamp(0, (canvas_height - height).max(0)),
            width,
            height,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanelLayout {
    pub placement: Placement,
    pub visible: bool,
    pub pinned: bool,
}

impl PanelLayout {
    fn initial(id: PanelId) -> Self {
        Self {
            placement: id.default_placement(),
            visible: matches!(id, PanelId::Server | PanelId::Notes),
            pinned: false,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OverlayLayout(BTreeMap<PanelId, PanelLayout>);

impl OverlayLayout {
    pub fn panel(&self, id: PanelId) -> PanelLayout {
        self.0
            .get(&id)
            .copied()
            .unwrap_or_else(|| PanelLayout::initial(id))
    }

    pub fn update(&mut self, id: PanelId, change: impl FnOnce(&mut PanelLayout)) {
        let mut panel = self.panel(id);
        change(&mut panel);
        self.0.insert(id, panel);
    }

    pub fn has_pinned_panels(&self) -> bool {
        PanelId::ALL.iter().any(|&id| {
            let panel = self.panel(id);
            panel.visible && panel.pinned
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panels_missing_from_the_saved_layout_get_their_defaults() {
        let layout: OverlayLayout = serde_json::from_str(r#"{"notes":{"placement":{"x":5,"y":6,"width":300,"height":200},"visible":false,"pinned":true}}"#).unwrap();
        assert_eq!(layout.panel(PanelId::Notes).placement.x, 5);
        assert!(!layout.panel(PanelId::Notes).visible);
        assert_eq!(
            layout.panel(PanelId::Server),
            PanelLayout::initial(PanelId::Server)
        );
    }

    #[test]
    fn only_visible_pinned_panels_count_as_pinned() {
        let mut layout = OverlayLayout::default();
        assert!(!layout.has_pinned_panels());
        layout.update(PanelId::History, |panel| panel.pinned = true);
        assert!(!layout.has_pinned_panels());
        layout.update(PanelId::History, |panel| panel.visible = true);
        assert!(layout.has_pinned_panels());
    }

    #[test]
    fn placements_stay_inside_the_screen_and_above_the_minimum_size() {
        let placement = Placement {
            x: 1900,
            y: -40,
            width: 400,
            height: 10,
        };
        assert_eq!(
            placement.kept_inside(1920, 1080),
            Placement {
                x: 1520,
                y: 0,
                width: 400,
                height: MINIMUM_PANEL_HEIGHT
            }
        );
    }

    #[test]
    fn layouts_round_trip_through_json() {
        let mut layout = OverlayLayout::default();
        layout.update(PanelId::Settings, |panel| panel.pinned = true);
        let json = serde_json::to_string(&layout).unwrap();
        assert_eq!(
            serde_json::from_str::<OverlayLayout>(&json).unwrap(),
            layout
        );
    }
}
