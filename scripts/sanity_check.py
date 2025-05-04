from pathlib import Path
from re import search
from sys import stderr
from json import dump, load

def check(content: str):
    errs = list()

    comment_disabled = "support.google.com/youtube/answer/9706180" in content
    if comment_disabled != ("จัดเรียงความคิดเห็น" not in content):
        errs.append("จัดเรียงความคิดเห็น")
    if comment_disabled != ("เพิ่มความคิดเห็น" not in content):
        errs.append("เพิ่มความคิดเห็น")
    if "ชอบ" not in content:
        errs.append("ชอบ")
    if "ไม่ชอบ" not in content:
        errs.append("ไม่ชอบ")
    if "แสดงการตอบกลับเพิ่มเติม" in content:
        errs.append("แสดงการตอบกลับเพิ่มเติม")
    # if "อ่านเพิ่มเติม" in content:
    #     errs.append("อ่านเพิ่มเติม")

    lines = content.splitlines()
    if any("poll-attachment" in line and "hidden" not in line for line in lines) \
        and not any(('icon="check-circle"' in chunk and "hidden" not in chunk for line in lines for chunk in line.split("<"))):
        errs.append("poll-attachment")

    return errs

if __name__ == "__main__":
    dirs = list(Path("archive").iterdir())
    invalids = dict()

    for fs in dirs:
        assert fs.exists()
        if not fs.is_file():
            continue

        content = fs.read_text()
        errs = check(content)
        if errs:
            print(f"{fs.name} is not valid: {errs}", file=stderr)
            invalids[fs.stem] = errs

    print(f"{len(invalids)}/{len(list(dirs))} invalid ({len(invalids)/len(list(dirs)):.2%})")

    with open("scripts/err/invalid.json", "w") as fs:
        dump(invalids, fs, ensure_ascii=False, indent=4)

    with open("scripts/err/invalid_ids.json", "w") as fs:
        ivs = list(invalids.keys())
        with open("data/post_ids.json", "r") as f: ids = load(f)
        ivs.sort(key=lambda x: ids.index(x))
        dump(ivs, fs, ensure_ascii=False, indent=4)
