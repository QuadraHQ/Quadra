use crate::state::Client;
use std::path::PathBuf;

pub enum StudioTarget {
    StartPage,
    Link(String),
    PlaceFile(PathBuf),
}

pub enum LaunchRequest {
    Roblox { link: Option<String> },
    Studio(StudioTarget),
}

const SIGN_IN_LINK_PREFIXES: [&str; 2] = ["roblox://single-sign-on/", "roblox-studio-auth:"];

impl LaunchRequest {
    pub fn sign_in_link(&self) -> Option<&str> {
        let link = match self {
            Self::Roblox { link } => link.as_deref()?,
            Self::Studio(StudioTarget::Link(link)) => link,
            Self::Studio(_) => return None,
        };
        SIGN_IN_LINK_PREFIXES
            .iter()
            .any(|prefix| link.starts_with(prefix))
            .then_some(link)
    }

    pub fn client(&self) -> Client {
        match self {
            Self::Roblox { .. } => Client::Player,
            Self::Studio(_) => Client::Studio,
        }
    }
}

#[cfg(test)]
mod sign_in_tests {
    use super::*;

    fn roblox(link: &str) -> LaunchRequest {
        LaunchRequest::Roblox {
            link: Some(link.to_owned()),
        }
    }

    #[test]
    fn only_links_that_finish_a_sign_in_are_sign_in_links() {
        let player = "roblox://single-sign-on/login?code=abc";
        let studio = LaunchRequest::Studio(StudioTarget::Link(
            "roblox-studio-auth:/?code=abc".to_owned(),
        ));

        assert_eq!(roblox(player).sign_in_link(), Some(player));
        assert!(studio.sign_in_link().is_some());
        assert!(
            roblox("roblox://experiences/start?placeId=1")
                .sign_in_link()
                .is_none()
        );
        assert!(
            LaunchRequest::Roblox { link: None }
                .sign_in_link()
                .is_none()
        );
    }
}
