import json
import os
import glob
import argparse

from git import Repo

from scripts.stats.collect_commits import (
    RunArgs,
    Result,
    parse_args,
    get_next_n_commits_from_base,
    switch_to_commit,
)


def extract_targets(target_str: str) -> list[str]:
    # "target": "ts_project rule //packages/compiler:compiler\n_strict_deps_test rule //packages/compiler:compiler_deps\nfilegroup rule //packages/compiler:files_for_docgen\n"
    targets = []
    for line in target_str.split("\n"):
        line = line.strip()
        if line == "":
            continue
        if line.startswith("//"):
            targets.append(line.split(" ")[0])
            continue
        parts = line.split(" ")
        if len(parts) >= 3:
            targets.append(parts[2])
    return targets


def main():
    args = parse_args()

    repo = Repo(args.repo_path)
    switch_to_commit(repo, args.base_commit)
    commits = get_next_n_commits_from_base(
        repo, args.default_branch, args.base_commit, args.n_commits
    )

    CHANGED_REBUILD_SETS_JSON_FILE_NAME = "changed_rebuild_sets.json"

    with open(
        os.path.join(args.result_dir, CHANGED_REBUILD_SETS_JSON_FILE_NAME), "r"
    ) as f:
        json_data = f.read()
    optimized_targets = set()
    for target, _, _ in json.loads(json_data):
        optimized_targets.add(target)

    n = 0
    optimized_commits = []
    for commit in commits:
        path = os.path.join(args.result_dir, f"{commit.hexsha}.json")
        if path.endswith(CHANGED_REBUILD_SETS_JSON_FILE_NAME):
            continue

        with open(path, "r") as f:
            json_data = f.read()
        result = Result.model_validate_json(json_data)
        n += 1

        for change in result.changes:
            if change.target is None:
                continue

            targets = extract_targets(change.target)
            for target in targets:
                if target in optimized_targets:
                    optimized_commits.append(result.commit_hash)
                    break

            if optimized_commits and optimized_commits[-1] == result.commit_hash:
                break

    print(f"Optimized Commits/Total: {len(optimized_commits)}/{n}")

    with open(os.path.join(args.result_dir, "optimized_commits.txt"), "w") as f:
        for commit in optimized_commits:
            f.write(f"{commit}\n")

    revertible_commits = []
    for commit in optimized_commits:
        switch_to_commit(repo, commit)
        try:
            repo.git.execute(
                f"git revert --no-commit {args.base_commit}".split(),
                stdout_as_string=True,
            )
            repo.git.execute("git revert --abort".split(), stdout_as_string=True)
            revertible_commits.append(commit)
            if len(revertible_commits) == 10:
                break
        except:
            repo.git.execute("git revert --abort".split(), stdout_as_string=True)
            break

    with open(os.path.join(args.result_dir, "revertible_commits.txt"), "w") as f:
        for commit in revertible_commits:
            f.write(f"{commit}\n")


if __name__ == "__main__":
    main()
