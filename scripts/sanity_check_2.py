from json import load, dump

with open("data/posts.json") as fs:
    posts = load(fs)

ids = list()
for post in posts:
    id = post['id']
    comments = post['comments']
    if comments is None:
        continue
    n = sum('![]()' in comment['content'] for comment in comments)
    if n > 0:
        print("Found", n, "blank images in", id)
        ids.append(id)

with open("scripts/err/invalid_ids2.json", "w") as fs:
    with open("data/post_ids.json", "r") as f: o = load(f)
    ids.sort(key=lambda x: o.index(x))
    dump(ids, fs, ensure_ascii=False, indent=4)
