use super::imported_theme::{ImportedTheme, parse_theme};
use crate::state::{Choice, Theme};
use std::sync::LazyLock;

const FILES: [(Theme, &str); 8] = [
    (
        Theme::GridLight,
        include_str!("../../data/themes/grid-light.css"),
    ),
    (
        Theme::GridDark,
        include_str!("../../data/themes/grid-dark.css"),
    ),
    (
        Theme::Monochrome,
        include_str!("../../data/themes/monochrome.css"),
    ),
    (Theme::Aurora, include_str!("../../data/themes/aurora.css")),
    (Theme::Ember, include_str!("../../data/themes/ember.css")),
    (Theme::Paper, include_str!("../../data/themes/paper.css")),
    (
        Theme::Phosphor,
        include_str!("../../data/themes/phosphor.css"),
    ),
    (Theme::Prism, include_str!("../../data/themes/prism.css")),
];

static THEMES: LazyLock<Vec<(Theme, ImportedTheme)>> = LazyLock::new(|| {
    FILES
        .iter()
        .filter_map(|&(theme, css)| match parse_theme(theme.label(), css) {
            Ok(parsed) => Some((theme, parsed)),
            Err(problem) => {
                eprintln!(
                    "The built-in theme {} couldn't be read: {problem}",
                    theme.label()
                );
                None
            }
        })
        .collect()
});

pub fn built_in(theme: Theme) -> &'static ImportedTheme {
    THEMES
        .iter()
        .find(|(built_in_theme, _)| *built_in_theme == theme)
        .or_else(|| THEMES.first())
        .map(|(_, parsed)| parsed)
        .expect("Quadra ships with built-in themes")
}

#[cfg(test)]
mod tests {
    use super::{FILES, parse_theme};
    use crate::state::{Choice, Theme};

    #[test]
    fn every_built_in_theme_parses_without_problems() {
        for theme in Theme::ALL {
            let (_, css) = FILES
                .iter()
                .find(|(file_theme, _)| file_theme == theme)
                .expect("every theme has a file");
            let parsed = parse_theme(theme.label(), css).expect("built-in themes parse");
            assert!(
                parsed.problems.is_empty(),
                "{}: {:?}",
                theme.label(),
                parsed.problems
            );
            assert_eq!(parsed.name, theme.label());
        }
    }
}
