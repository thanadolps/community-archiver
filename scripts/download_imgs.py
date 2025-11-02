from json import load
from os import system
from pathlib import Path
from urllib.parse import quote, unquote
from re import compile

posts = load(open("data/posts.json"))


attachment = [img for post in posts if post['content_attachment'] is not None for img in post['content_attachment']['images']]
attachment = [a.split('?')[0] for a in attachment]
attachment = set(attachment)
if "https://i.ytimg.com/img/no_thumbnail.jpg" in attachment:
    attachment.remove("https://i.ytimg.com/img/no_thumbnail.jpg")


# Excluded downloaded images
downloaded = set(unquote(d.stem) for d in Path("archive_imgs").iterdir() if d.is_file() and d.stat().st_size > 0)
to_download = [a for a in attachment if a.split('/')[-1] not in downloaded]
print(f"to_download={to_download}")


# Download the attachments
names = set()
pat = compile(r"(.+)=s\d+")

for i, url in enumerate(to_download):
    print(f"Downloading {url} ({i+1}/{len(to_download)})")
    filename = quote(url.replace("https://", "").split('/', 1)[1], safe="")
    if filename in names:
        print(f"{filename} already processed")
        exit(1)
    names.add(filename)
    filepath = f"archive_imgs/{filename}"

    status = system(f"wget -O {filepath} {url}")
    if status != 0:
        print(f"command return with status {status}")
    
    g = pat.match(url)
    if g is None:
        exit(1)
    url = g.group(0)
    print(f"Failed, trying {url} instead")
    status = system(f"wget -O {filepath} {url}")
    if status != 0:
        print(f"command return with status {status}")
    url = g.group(1)
    print(f"Failed, trying {url} instead")
    status = system(f"wget -O {filepath} {url}")
    if status != 0:
        print(f"command return with status {status}")
        exit(status)

