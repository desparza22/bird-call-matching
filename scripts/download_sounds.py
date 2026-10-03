import json
import os
from datetime import datetime

import requests

for i in range(1, 37):
    list_file = f"sound-lists/xeno-canto-list-{i}.json"
    print(f"list_file: {list_file}")
    print(datetime.now())
    with open(list_file, "r") as f:
        data = json.load(f)
        for idx, recording in enumerate(data["recordings"]):
            if recording["file"] is None:
                continue
            if i == 1 and idx % 10 == 0:
                print(f"First list file, recording {idx}")
                print(datetime.now())

            write_to = f"downloads/{recording['id']}"
            if os.path.isdir(write_to):
                continue
            os.mkdir(write_to)

            # Send a GET request to the URL
            response = requests.get(recording["file"])

            # Check if the request was successful (status code 200)
            if response.status_code == 200:
                # Open a local file in write-binary mode ('wb')
                with open(f"{write_to}/sound.wav", "wb") as file:
                    file.write(response.content)
                with open(f"{write_to}/metadata.json", "w") as file:
                    file.write(json.dumps(recording))

            else:
                print(f"Failed to download file. Status code: {response.status_code}")
