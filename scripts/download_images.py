import json
import os
from pathlib import Path

import requests

birds_root = "/users/diegoesparza/cs_ventures/current_projects/bird-call-matching/bird_call_matching/public/birds"

for bird_dir in os.listdir(birds_root):
    # skip birds we've already processed
    images_dir_path = birds_root + "/" + bird_dir + "/images"
    if not os.path.isdir(images_dir_path):
        os.mkdir(images_dir_path)
    images_metadata_path = images_dir_path + "/metadata.json"
    if Path(images_metadata_path).exists():
        print(f"skipping {bird_dir}")
        continue

    # read bird data
    with open(birds_root + "/" + bird_dir + "/metadata.json") as metadata_f:
        bird_metadata = json.load(metadata_f)

    gen = bird_metadata["gen"]
    sp = bird_metadata["sp"]
    en = bird_metadata["en"]

    # find taxon_id
    autocomplete_url = (
        f"https://api.inaturalist.org/v1/taxa/autocomplete?q={gen}%20{sp}"
    )
    autocomplete_response = requests.get(autocomplete_url)
    if autocomplete_response.status_code != 200:
        raise ValueError(autocomplete_response)
    autocomplete_data = json.loads(autocomplete_response.text)
    if autocomplete_data["total_results"] == 0:
        raise ValueError(f"no results for {autocomplete_url}")
    taxon_id = autocomplete_data["results"][0]["id"]

    # find urls
    # NOTE: I think there's an observations api that takes multiple taxon_ids, probably much faster and better on endpoint, forgot about it until script was mostly done running.
    observations_url = f"https://api.inaturalist.org/v1/observations?taxon_id={taxon_id}&photo_license=cc-by,cc-by-sa,cc0&per_page=1"
    observations_response = requests.get(observations_url)
    if observations_response.status_code != 200:
        raise ValueError(observations_response)
    observations_data = json.loads(observations_response.text)
    if observations_data["total_results"] == 0:
        raise ValueError(f"no results for {observations_url}")
    observation = observations_data["results"][0]["taxon"]["default_photo"]
    url = observation.get("url", None)
    medium_url = observation.get("medium_url", None)
    square_url = observation.get("square_url", None)

    # write image url metadata
    images_metadata = {
        "gen": gen,
        "sp": sp,
        "en": en,
        "url": url,
        "medium_url": medium_url,
        "square_url": square_url,
    }
    missing_urls = [
        u
        for u in [("url", url), ("medium_url", medium_url), ("square_url", square_url)]
        if not u[1]
    ]
    missing_urls_context = "".join([" (missing " + u[0] + ")" for u in missing_urls])
    print(f"Writing {en} {gen} {sp}: {medium_url}{missing_urls_context}")
    with open(images_metadata_path, "w") as images_metadata_f:
        json.dump(images_metadata, images_metadata_f)
