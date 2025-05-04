from json import load
from os import system
import re


with open("data/emote_mapping.json") as fs:
    mapping = load(fs)

for code, name in mapping.items():
    url =  f"https://yt3.ggpht.com/{code}s240-c-k-nd" # use 240px images
    system(f"wget {url} -O emote/{name}.png")
