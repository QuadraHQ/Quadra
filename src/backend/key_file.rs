use std::fmt;

const GROUP_START: char = '[';
const COMMENT_START: char = '#';

#[derive(Default)]
pub struct KeyFile {
    lines: Vec<String>,
}

impl KeyFile {
    pub fn parse(contents: &str) -> Self {
        Self {
            lines: contents.lines().map(str::to_owned).collect(),
        }
    }

    pub fn value(&self, group: &str, key: &str) -> Option<&str> {
        let position = self.key_position(group, key)?;
        let (_, value) = self.lines[position].split_once('=')?;
        Some(value.trim())
    }

    pub fn set_value(&mut self, group: &str, key: &str, value: &str) {
        let line = format!("{key}={value}");
        if let Some(position) = self.key_position(group, key) {
            self.lines[position] = line;
            return;
        }

        match self.group_lines(group) {
            Some(lines) => {
                let last_entry = self.lines[lines.clone()]
                    .iter()
                    .rposition(|line| !line.trim().is_empty())
                    .map_or(lines.start, |offset| lines.start + offset + 1);
                self.lines.insert(last_entry, line);
            }
            None => {
                if self
                    .lines
                    .last()
                    .is_some_and(|line| !line.trim().is_empty())
                {
                    self.lines.push(String::new());
                }
                self.lines.push(format!("{GROUP_START}{group}]"));
                self.lines.push(line);
            }
        }
    }

    pub fn remove(&mut self, group: &str, key: &str) {
        if let Some(position) = self.key_position(group, key) {
            self.lines.remove(position);
        }
    }

    fn group_lines(&self, group: &str) -> Option<std::ops::Range<usize>> {
        let header = format!("{GROUP_START}{group}]");
        let header_position = self.lines.iter().position(|line| line.trim() == header)?;
        let start = header_position + 1;
        let end = self.lines[start..]
            .iter()
            .position(|line| line.trim_start().starts_with(GROUP_START))
            .map_or(self.lines.len(), |offset| start + offset);
        Some(start..end)
    }

    fn key_position(&self, group: &str, key: &str) -> Option<usize> {
        let lines = self.group_lines(group)?;
        let start = lines.start;
        self.lines[lines]
            .iter()
            .position(|line| {
                let line = line.trim_start();
                !line.starts_with(COMMENT_START)
                    && line
                        .split_once('=')
                        .is_some_and(|(name, _)| name.trim() == key)
            })
            .map(|offset| start + offset)
    }
}

impl fmt::Display for KeyFile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for line in &self.lines {
            writeln!(formatter, "{line}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::KeyFile;

    #[test]
    fn reads_values_from_the_right_group() {
        let file = KeyFile::parse(
            "# made by hand\n[Default Applications]\nx-scheme-handler/roblox=a.desktop\n\n[Added Associations]\nx-scheme-handler/roblox = b.desktop;a.desktop;\n",
        );
        assert_eq!(
            file.value("Default Applications", "x-scheme-handler/roblox"),
            Some("a.desktop")
        );
        assert_eq!(
            file.value("Added Associations", "x-scheme-handler/roblox"),
            Some("b.desktop;a.desktop;")
        );
        assert_eq!(file.value("Missing", "x-scheme-handler/roblox"), None);
    }

    #[test]
    fn keeps_comments_and_other_entries_when_changing_values() {
        let mut file = KeyFile::parse(
            "# keep me\n[Default Applications]\ntext/html=firefox.desktop\n\n[Other]\nkey=value\n",
        );
        file.set_value(
            "Default Applications",
            "x-scheme-handler/roblox",
            "quadra.desktop",
        );
        file.set_value(
            "Added Associations",
            "x-scheme-handler/roblox",
            "quadra.desktop;",
        );
        file.set_value("Default Applications", "text/html", "chromium.desktop");
        file.remove("Other", "key");

        assert_eq!(
            file.to_string(),
            "# keep me\n[Default Applications]\ntext/html=chromium.desktop\nx-scheme-handler/roblox=quadra.desktop\n\n[Other]\n\n[Added Associations]\nx-scheme-handler/roblox=quadra.desktop;\n"
        );
    }
}
