import os
import subprocess as sp

from git import Repo

from scripts.stats.test_incre_build import find_prev_commit, parse_args


def get_prev_commit(
    repo: Repo,
    commit: str,
    base_commit: str,
):
    prev_commit = find_prev_commit(repo, commit)
    need_revert = True
    if prev_commit == base_commit:
        prev_commit = find_prev_commit(repo, base_commit)
        need_revert = False
    return prev_commit, need_revert


def main():
    args = parse_args()

    repo = Repo(args.repo_path)

    with open(os.path.join(args.result_dir, "revertible_commits.txt"), "r") as f:
        commits = [line.strip().split(" ")[0] for line in f.readlines()]

    with open(os.path.join(args.result_dir, "revertible_prev_commits.txt"), "w") as f:
        for commit in commits:
            if commit.strip() == "":
                continue
            prev_commit, need_revert = get_prev_commit(
                repo,
                commit,
                args.base_commit,
            )
            f.write(f"{commit} {prev_commit} {int(need_revert)}\n")


if __name__ == "__main__":
    main()
