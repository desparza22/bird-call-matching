THESE STEPS AREN'T REPRODUCIBLE, SOME EXTRA MANUAL STEPS WERE MIXED IN

Bird lists generated with something like the below (not entirely, some additional fields, and also downloaded every page whereas the below just downloads the first):
curl -G "https://xeno-canto.org/api/3/recordings" --data-urlencode "key=75646ba2fb45f35020ea0ec77a1c030c9c94c278" --data-urlencode "query=grp:birds q:A cnt:\"united states\" len:\">3\" len:\"<15\""

download_sounds.py took those metadata files and downloaded the sounds

restructure_files.py restructured the downloaded files to organize them by bird
