use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Identifier {
    Number(u64),
    Text(String),
}

impl Ord for Identifier {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Number(a), Self::Number(b)) => a.cmp(b),
            (Self::Number(_), Self::Text(_)) => Ordering::Less,
            (Self::Text(_), Self::Number(_)) => Ordering::Greater,
            (Self::Text(a), Self::Text(b)) => a.cmp(b),
        }
    }
}

impl PartialOrd for Identifier {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    core: [u64; 3],
    prerelease: Vec<Identifier>,
}

impl Version {
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.strip_prefix('v').unwrap_or(text);
        let text = text.split('+').next()?;
        let (core, prerelease) = match text.split_once('-') {
            Some((core, prerelease)) => (core, Some(prerelease)),
            None => (text, None),
        };

        let mut numbers = core.split('.').map(|number| number.parse().ok());
        let version_core = [numbers.next()??, numbers.next()??, numbers.next()??];
        if numbers.next().is_some() {
            return None;
        }

        let prerelease = match prerelease {
            Some(prerelease) => prerelease
                .split('.')
                .map(identifier)
                .collect::<Option<Vec<_>>>()?,
            None => Vec::new(),
        };
        Some(Self {
            core: version_core,
            prerelease,
        })
    }

    pub fn is_prerelease(&self) -> bool {
        !self.prerelease.is_empty()
    }
}

fn identifier(text: &str) -> Option<Identifier> {
    if text.is_empty() {
        return None;
    }
    Some(match text.parse() {
        Ok(number) => Identifier::Number(number),
        Err(_) => Identifier::Text(text.to_owned()),
    })
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.core.cmp(&other.core).then_with(|| {
            match (self.prerelease.is_empty(), other.prerelease.is_empty()) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                (false, false) => self.prerelease.cmp(&other.prerelease),
            }
        })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::Version;

    fn version(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    #[test]
    fn orders_versions_like_semver() {
        let ascending = [
            "0.50.0-alpha.1",
            "0.50.0-alpha.2",
            "0.50.0-alpha.10",
            "0.50.0-beta",
            "0.50.0",
            "0.50.1",
            "0.51.0-alpha.1",
            "1.0.0",
        ];
        for pair in ascending.windows(2) {
            assert!(
                version(pair[0]) < version(pair[1]),
                "{} < {}",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn accepts_tags_and_ignores_build_metadata() {
        assert_eq!(version("v0.50.0-alpha.2"), version("0.50.0-alpha.2+abc"));
        assert!(version("v0.50.0-alpha.2").is_prerelease());
        assert!(!version("0.50.0").is_prerelease());
    }

    #[test]
    fn rejects_text_that_is_not_a_version() {
        for text in [
            "",
            "1.2",
            "1.2.3.4",
            "a.b.c",
            "1.2.3-",
            "1.2.3-a..b",
            "latest",
        ] {
            assert!(Version::parse(text).is_none(), "{text}");
        }
    }
}
