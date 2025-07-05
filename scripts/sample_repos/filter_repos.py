import json
import glob

from pprint import pprint

res = set()
for path in glob.glob("repos/*.json"):
    with open(path, "r") as f:
        data = f.read()
        if data.strip() == "":
            continue
        data = json.loads(data)

    if data["fork"] or data["archived"] or data["disabled"]:
        continue

    if data["stargazers_count"] < 100 or data["pushed_at"] < "2025-01-01T00:00:00Z" or data["size"] < 1000:
        continue

    res.add(
        (data["full_name"], data["stargazers_count"], data["pushed_at"], data["size"])
    )

res = list(res)
# res.sort(key=lambda x: (x[1], x[3]))
res.sort(key=lambda x: (x[3], x[1]))

with open("filtered_repos.jsonl", "w") as f:
    for name, stars, pushed_at, size in res:
        f.write(
            json.dumps(
                {
                    "repo_name": name,
                    "stars": stars,
                    "pushed_at": pushed_at,
                    "size": size,
                },
                ensure_ascii=False,
            )
            + "\n"
        )

print(len(res))
