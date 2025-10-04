from json import load
from os import system
from pathlib import Path

posts = load(open("data/posts.json"))


attachment = [img for post in posts if post['content_attachment'] is not None for img in post['content_attachment']['images']]
attachment = [a.split('?')[0] for a in attachment]
attachment = set(attachment)
if "https://i.ytimg.com/img/no_thumbnail.jpg" in attachment:
    attachment.remove("https://i.ytimg.com/img/no_thumbnail.jpg")


# Excluded downloaded images
downloaded = set(d.stem for d in Path("archive_imgs").iterdir() if d.is_file())
to_download = [a for a in attachment if a.split('/')[-1] not in downloaded]

# Download the attachments
for i, url in enumerate(to_download):
    print(f"Downloading {url} ({i+1}/{len(to_download)})")
    filename = f"archive_imgs/{url.split('/')[-1]}"
    system(f"wget -O {filename} {url}")


