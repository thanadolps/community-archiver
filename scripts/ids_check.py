from pathlib import Path
from json import load

with open("data/post_ids.json") as fs:
    source_list = load(fs)
source = set(source_list)

dest = set()
for fs in Path("archive").iterdir():
    if fs.is_dir():
        continue
    id = fs.stem
    dest.add(id)


print("Extra:", dest - source)
print("Missing:", sorted(source - dest, key=lambda x: source_list.index(x)))
