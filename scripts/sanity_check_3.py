from json import load
import re


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


with open("data/posts.json") as fs:
    posts = load(fs)

images = set()
for post in posts:
    id = post['id']
    comments = post['comments']
    if comments is None:
        continue
    for comment in comments:
        content = comment['content']

        # Find all image links in the comment
        imgs = re.findall(r'<img src="([^"]+)">', content)
        images.update(imgs)

print("Found", len(images), "unique images")

for img in images:
    code = img.split("/")[-1].split("=")[0] + "="
    if code in mapping:
        name = mapping[code]
    else:
        print("Unknown emote:", code)