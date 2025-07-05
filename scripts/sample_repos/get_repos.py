import os
import time
import json

from pprint import pprint
from github import Github, Auth


def get_token():
    with open(os.path.join(os.path.dirname(__file__), "token.txt"), "r") as f:
        return f.read().strip()


AUTH = Auth.Token(get_token())
GH = Github(auth=AUTH)


def read_repositories(path):
    with open(path, "r") as f:
        for line in f:
            yield json.loads(line)["repo_name"]


def get_repo(repo_name):
    try:
        repo = GH.get_repo(repo_name)
        return json.dumps(repo.raw_data, indent=2, ensure_ascii=False)
    except Exception as e:
        print(f"Error fetching {repo_name}: {e}")
        return None


os.makedirs("repos", exist_ok=True)
repos = sorted(read_repositories("bq-res.json"))
for i, repo in enumerate(repos):
    if os.path.exists(f"repos/{i:04d}.json"):
        print(f"Skipping {i:04d} as it already exists.")
        continue
    with open(f"repos/{i:04d}.json", "w") as f:
        res = get_repo(repo)
        if res is None:
            continue
        pprint(res)
        f.write(res)
    time.sleep(0.2)