import json
import os
from pathlib import Path

public_assets_root = "/Users/diegoesparza/CS_Ventures/current_projects/bird-call-matching/bird_call_matching/public"
sounds_root = public_assets_root + "/sounds"
birds_root = public_assets_root + "/birds"

names = {}
dirs_written = 0


def find_dir_for_name(name):
    global dirs_written
    if name in names:
        return names[name]

    dir_for_name = f"{birds_root}/{dirs_written}"
    # os.mkdir(dir_for_name)

    names[name] = dir_for_name
    dirs_written += 1
    return dir_for_name


for id in os.listdir(sounds_root):
    metadata_path = f"{sounds_root}/{id}/metadata.json"
    if not Path(metadata_path).is_file():
        print(f"Missing {metadata_path}")
        continue

    with open(metadata_path, "r") as metadata_f:
        metadata = json.load(metadata_f)

    dir_for_name = find_dir_for_name(metadata["en"])
    src = f"{sounds_root}/{metadata['id']}"
    dst = f"{dir_for_name}/{metadata['id']}"
    Path(src).rename(dst)

# print(set(names))
