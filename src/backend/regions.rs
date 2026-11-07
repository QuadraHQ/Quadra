use super::paths;
use crate::state::{Region, country_and_city_of};
use anyhow::Context;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const ROVALRA_API: &str = "https://apis.rovalra.com";
const USER_AGENT: &str = "Quadra";
const KEEP_REGIONS_FOR: Duration = Duration::from_secs(24 * 60 * 60);

pub struct PickedServer {
    pub job_id: String,
    pub location: Option<String>,
}

#[derive(Deserialize)]
struct Datacenter {
    #[serde(default)]
    inactive: bool,
    location: Option<DatacenterLocation>,
}

#[derive(Deserialize)]
struct DatacenterLocation {
    country: Option<String>,
    country_name: Option<String>,
    city: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct CachedRegions {
    fetched_at: u64,
    regions: Vec<Region>,
}

#[derive(Deserialize)]
struct ServersInRegion {
    #[serde(default)]
    servers: Vec<ServerInRegion>,
}

#[derive(Deserialize)]
struct ServerInRegion {
    server_id: String,
    city: Option<String>,
    country: Option<String>,
}

pub fn available() -> anyhow::Result<Vec<Region>> {
    let cached = read_cache();
    if let Some(cached) = &cached
        && seconds_since_epoch().saturating_sub(cached.fetched_at) < KEEP_REGIONS_FOR.as_secs()
    {
        return Ok(cached.regions.clone());
    }

    match fetch_regions() {
        Ok(regions) if !regions.is_empty() => {
            write_cache(&regions);
            Ok(regions)
        }
        Ok(_) => Ok(cached.map(|cached| cached.regions).unwrap_or_default()),
        Err(error) => match cached {
            Some(cached) => {
                eprintln!("Using the saved region list, the new one is unavailable: {error:#}");
                Ok(cached.regions)
            }
            None => Err(error),
        },
    }
}

pub fn pick_server(
    place_id: u64,
    region_key: &str,
    avoided_job_ids: &[String],
) -> anyhow::Result<Option<PickedServer>> {
    let (country, city) = country_and_city_of(region_key);
    let mut request = ureq::get(format!("{ROVALRA_API}/v1/servers/region"))
        .header("User-Agent", USER_AGENT)
        .query("place_id", place_id.to_string())
        .query("country", country)
        .query("cursor", "0");
    if let Some(city) = city {
        request = request.query("city", city);
    }

    let found: ServersInRegion = request
        .call()
        .and_then(|mut response| response.body_mut().read_json())
        .context("Couldn't ask RoValra for servers in the region")?;

    let unvisited: Vec<&ServerInRegion> = found
        .servers
        .iter()
        .filter(|server| !avoided_job_ids.contains(&server.server_id))
        .collect();
    let candidates: Vec<&ServerInRegion> = if unvisited.is_empty() {
        found.servers.iter().collect()
    } else {
        unvisited
    };

    Ok(any_of(&candidates).map(|server| PickedServer {
        job_id: server.server_id.clone(),
        location: location_of(server),
    }))
}

fn fetch_regions() -> anyhow::Result<Vec<Region>> {
    let datacenters: Vec<Datacenter> = get_json(&format!("{ROVALRA_API}/v1/datacenters/list"))?;
    let cities: BTreeSet<(String, String, String)> = datacenters
        .into_iter()
        .filter(|datacenter| !datacenter.inactive)
        .filter_map(|datacenter| {
            let location = datacenter.location?;
            let country_code = location.country.filter(|code| !code.is_empty())?;
            let city = location.city.filter(|city| !city.is_empty())?;
            let country_name = location
                .country_name
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| country_code.clone());
            Some((country_name, country_code, city))
        })
        .collect();

    let mut regions = Vec::new();
    let mut cities = cities.into_iter().peekable();
    while let Some((country_name, country_code, first_city)) = cities.next() {
        let mut cities_in_country = vec![first_city];
        while let Some((_, _, city)) =
            cities.next_if(|(_, next_country_code, _)| *next_country_code == country_code)
        {
            cities_in_country.push(city);
        }

        regions.push(Region {
            country_name: country_name.clone(),
            country_code: country_code.clone(),
            city: None,
        });
        if cities_in_country.len() > 1 {
            regions.extend(cities_in_country.into_iter().map(|city| Region {
                country_name: country_name.clone(),
                country_code: country_code.clone(),
                city: Some(city),
            }));
        }
    }

    Ok(regions)
}

fn location_of(server: &ServerInRegion) -> Option<String> {
    let parts: Vec<&str> = [&server.city, &server.country]
        .into_iter()
        .flatten()
        .map(String::as_str)
        .filter(|part| !part.is_empty())
        .collect();
    (!parts.is_empty()).then(|| parts.join(", "))
}

fn any_of<'a>(servers: &[&'a ServerInRegion]) -> Option<&'a ServerInRegion> {
    if servers.is_empty() {
        return None;
    }
    let varying_number = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since_epoch| since_epoch.subsec_nanos() as usize)
        .unwrap_or_default();
    Some(servers[varying_number % servers.len()])
}

fn read_cache() -> Option<CachedRegions> {
    let contents = fs::read_to_string(paths::regions_cache_file()).ok()?;
    serde_json::from_str(&contents).ok()
}

fn write_cache(regions: &[Region]) {
    let path = paths::regions_cache_file();
    let cached = CachedRegions {
        fetched_at: seconds_since_epoch(),
        regions: regions.to_vec(),
    };
    let written = path
        .parent()
        .map_or(Ok(()), fs::create_dir_all)
        .map_err(anyhow::Error::from)
        .and_then(|()| Ok(serde_json::to_string(&cached)?))
        .and_then(|contents| Ok(fs::write(&path, contents)?));
    if let Err(error) = written {
        eprintln!(
            "Couldn't save the region list to {}: {error:#}",
            path.display()
        );
    }
}

fn seconds_since_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since_epoch| since_epoch.as_secs())
        .unwrap_or_default()
}

fn get_json<T: DeserializeOwned>(url: &str) -> anyhow::Result<T> {
    ureq::get(url)
        .header("User-Agent", USER_AGENT)
        .call()
        .and_then(|mut response| response.body_mut().read_json())
        .with_context(|| format!("Couldn't get {url}"))
}
