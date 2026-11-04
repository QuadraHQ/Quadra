"""Region-preferred server joining through RoValra's public API.

`/v1/datacenters/list` names the locations, `/v1/servers/counts?place_id=`
says where a game has servers, and `/v1/servers/region?place_id=&country=`
lists servers there (their `server_id` is Roblox's gameInstanceId). None of
it needs a Roblox sign-in; what RoValra sees is the place id and the region
asked for, which is why the feature is opt-in. Schemas checked against the
live API on 2026-11-01. The approach comes from AppleBlox's region selector."""

import json
import logging
import os
import random
import time
import urllib.parse
import urllib.request
from pathlib import Path

log = logging.getLogger("quadra.region")

API = "https://apis.rovalra.com"
CACHE_SECONDS = 24 * 3600
MAX_BYTES = 8 * 1024 * 1024


def _get(path, query=None, opener=urllib.request.urlopen):
    url = API + path + ("?" + urllib.parse.urlencode(query) if query else "")
    request = urllib.request.Request(url, headers={"User-Agent": "Quadra"})
    with opener(request, timeout=10) as response:
        return json.loads(response.read(MAX_BYTES))


def parse_locations(data):
    """[(country_code, country_name, city)] of active datacenters, sorted."""
    found = set()
    for entry in data if isinstance(data, list) else []:
        if not isinstance(entry, dict) or entry.get("inactive"):
            continue
        place = entry.get("location")
        if (
            not isinstance(place, dict)
            or not place.get("country")
            or not place.get("city")
        ):
            continue
        found.add(
            (
                str(place["country"]),
                str(place.get("country_name") or place["country"]),
                str(place["city"]),
            )
        )
    return sorted(found, key=lambda item: (item[1], item[2]))


def locations(cache_dir, *, opener=urllib.request.urlopen):
    """Datacenter locations, cached for a day; the stale copy if offline."""
    path = Path(cache_dir) / "rovalra-datacenters.json"
    cached = None
    try:
        cached = json.loads(path.read_text())
        if time.time() - cached["at"] < CACHE_SECONDS:
            return [tuple(item) for item in cached["locations"]]
    except (OSError, ValueError, KeyError, TypeError):
        cached = None
    try:
        found = parse_locations(_get("/v1/datacenters/list", opener=opener))
    except Exception as error:
        log.info("Datacenter list unavailable: %s", error)
        found = []
    if found:
        try:
            path.parent.mkdir(parents=True, exist_ok=True)
            temporary = path.with_suffix(".tmp")
            temporary.write_text(json.dumps({"at": time.time(), "locations": found}))
            os.replace(temporary, path)
        except OSError:
            pass
        return found
    return [tuple(item) for item in cached["locations"]] if cached else []


def region_key(country, city=None):
    return f"{country}:{city}" if city else country


def split_region(key):
    """'DE:Frankfurt am Main' -> ('DE', 'Frankfurt am Main'); 'DE' -> ('DE', None)."""
    country, _, city = (key or "").partition(":")
    return (country or None), (city or None)


def pick_server(place_id, key, *, avoid=(), opener=urllib.request.urlopen):
    """A server_id in the region, or None. Servers in `avoid` (recent joins)
    are used only when nothing else is there."""
    country, city = split_region(key)
    if not country:
        return None
    query = {"place_id": int(place_id), "country": country, "cursor": 0}
    if city:
        query["city"] = city
    try:
        data = _get("/v1/servers/region", query, opener=opener)
    except Exception as error:
        log.info("Region server lookup failed: %s", error)
        return None
    servers = [
        s
        for s in data.get("servers", [])
        if isinstance(s, dict) and isinstance(s.get("server_id"), str)
    ]
    # Not full when RoValra says so; most responses carry no player counts.
    servers = [
        s
        for s in servers
        if not (
            isinstance(s.get("playing"), int)
            and isinstance(s.get("max_players"), int)
            and s["playing"] >= s["max_players"]
        )
    ]
    fresh = [s for s in servers if s["server_id"] not in avoid]
    chosen = random.choice(fresh or servers) if (fresh or servers) else None
    return chosen["server_id"] if chosen else None


def parse_place(text):
    """A place id from what the user typed: digits or a roblox.com/games/ link."""
    text = (text or "").strip()
    if text.isdigit():
        return int(text)
    marker = "/games/"
    if marker in text:
        rest = text.split(marker, 1)[1].split("/", 1)[0]
        if rest.isdigit():
            return int(rest)
    return None
