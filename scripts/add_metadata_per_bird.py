import json
import os
from pathlib import Path

bird_assets_root = "/Users/diegoesparza/CS_Ventures/current_projects/bird-call-matching/bird_call_matching/public/birds"

for bird in os.listdir(bird_assets_root):
    bird_dir = f"{bird_assets_root}/{bird}"
    metadata_path = f"{bird_dir}/metadata.json"
    if Path(metadata_path).is_file():
        continue

    sound_sample = os.listdir(bird_dir)[0]
    sample_metadata_json = f"{bird_dir}/{sound_sample}/metadata.json"
    with open(sample_metadata_json, "r") as sample_metadata_f:
        sample_metadata = json.load(sample_metadata_f)

    bird_metadata = {
        "gen": sample_metadata["gen"],
        "sp": sample_metadata["sp"],
        "en": sample_metadata["en"],
    }

    with open(metadata_path, "w") as metadata_f:
        metadata_f.write(json.dumps(bird_metadata))
