from json import load
import re
from pathlib import Path


mapping = {}
try:
    with open("data/emote_mapping.json") as fs:
        mapping |= load(fs)
except FileNotFoundError:
    print("WARN: No emote mapping file found")
    pass
try:
    with open("data/emote_mapping_default.json") as fs:
        mapping |= load(fs)
except FileNotFoundError:
    print("WARN: No emote mapping default file found")
    pass

reverse_mapping = {v: k for k, v in mapping.items()}

with open("data/posts.json") as fs:
    posts = load(fs)

images = set()
emotes = dict[str, list[str]]()
for post in posts:
    comments = post["comments"]
    if comments is None:
        continue

    assert type(comments) is list

    while len(comments) > 1:
        comment = comments.pop()
        if "replies" in comment:
            comments.extend(comment["replies"])

        content: str = comment["content"]

        # Find all image links in the comment
        imgs = re.findall(r'<img src="([^"]+)">', content)
        images.update(imgs)

        # Find emoji reference
        emoji = re.findall(r":_(\w+):", content)
        for e in emoji:
            emotes.setdefault(e, []).append(comment["url"])

print("Found", len(images), "unique images")
print("Found", len(emotes), "unique emotes")

unknown = set()
for img in images:
    code = img.split("/")[-1].split("=")[0] + "="
    if code in mapping:
        name = mapping[code]
    else:
        unknown.add(code)
        print("Unknown image:", code)


for emoji, urls in emotes.items():
    if emoji not in reverse_mapping:
        unknown.add(emoji)
        print("Unknown emoji:", emoji)
        print(
            "(unknown emoji have high false positive rate, please review manually at:)"
        )
        for url in urls:
            print(f"\t- https://www.youtube.com/{url}")


if len(unknown) > 0:
    exit(1)
