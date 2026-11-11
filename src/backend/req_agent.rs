use anyhow::Context;
use serde::de::DeserializeOwned;
use std::sync::LazyLock;
use std::time::Duration;
use ureq::Agent;

const USER_AGENT: &str = "Quadra";
const API_TIMEOUT: Duration = Duration::from_secs(15);
const API_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const DOWNLOAD_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const DOWNLOAD_RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);

static API_AGENT: LazyLock<Agent> = LazyLock::new(|| {
    Agent::new_with_config(
        Agent::config_builder()
            .user_agent(USER_AGENT)
            .timeout_global(Some(API_TIMEOUT))
            .timeout_connect(Some(API_CONNECT_TIMEOUT))
            .build(),
    )
});

static DOWNLOAD_AGENT: LazyLock<Agent> = LazyLock::new(|| {
    Agent::new_with_config(
        Agent::config_builder()
            .user_agent(USER_AGENT)
            .timeout_connect(Some(DOWNLOAD_CONNECT_TIMEOUT))
            .timeout_recv_response(Some(DOWNLOAD_RESPONSE_TIMEOUT))
            .build(),
    )
});

pub fn api_agent() -> &'static Agent {
    &API_AGENT
}

pub fn download_agent() -> &'static Agent {
    &DOWNLOAD_AGENT
}

pub fn get_json<T: DeserializeOwned>(url: &str) -> anyhow::Result<T> {
    api_agent()
        .get(url)
        .call()
        .and_then(|mut response| response.body_mut().read_json())
        .with_context(|| format!("Couldn't get {url}"))
}
