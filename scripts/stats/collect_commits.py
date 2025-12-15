import argparse
from pydantic import BaseModel, Field
from git import Repo


class RunArgs(BaseModel):
    repo_path: str
    default_branch: str
    base_commit: str
    n_commits: int = Field(default=100)


def parse_args():
    argparser = argparse.ArgumentParser()
    argparser.add_argument(
        "-c",
        "--config",
        type=str,
        required=True,
        help="Path to the configuration file",
    )
    args = argparser.parse_args()
    with open(args.config, "r") as f:
        config_data = f.read()
    return RunArgs.model_validate_json(config_data)


def switch_to_commit(repo: Repo, commit_hash: str):
    repo.remote().fetch(commit_hash)
    repo.git.checkout(commit_hash)


def get_next_n_commits_from_base(
    repo: Repo, default_branch: str, base_commit: str, n: int
):
    repo.remote().fetch(default_branch)
    commits = list(
        repo.iter_commits(f"{base_commit}..origin/{default_branch}")
    )[-n:]
    commits.reverse()
    return commits


def main():
    args = parse_args()
    print(f"Collecting commits from repository at: {args.repo_path}")

    repo = Repo(args.repo_path)
    switch_to_commit(repo, args.base_commit)
    commits = get_next_n_commits_from_base(
        repo, args.default_branch, args.base_commit, args.n_commits
    )
    print(commits)


if __name__ == "__main__":
    main()
