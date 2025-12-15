import os
import argparse
import subprocess

from pydantic import BaseModel, Field
from git import Repo, Tree


class RunArgs(BaseModel):
    repo_path: str
    default_branch: str
    base_commit: str
    result_dir: str
    n_commits: int = Field(default=100)
    prerun: str = Field(default=None)
    postrun: str = Field(default=None)
    exclude_paths: list[str] = Field(default_factory=list)


class Change(BaseModel):
    old_path: str
    new_path: str
    target: str | None


class Result(BaseModel):
    commit_hash: str
    changes: list[Change]


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
    commits = list(repo.iter_commits(f"{base_commit}..origin/{default_branch}"))[-n:]
    commits.reverse()
    return commits


def get_changed_files_between_commits(
    repo: Repo, old_commit_hash: str, new_commit_hash: str
):
    old_commit = repo.commit(old_commit_hash)
    new_commit = repo.commit(new_commit_hash)
    diff_index = old_commit.diff(new_commit)

    changed_files = []
    for diff in diff_index:
        if diff.a_path != diff.b_path:
            changed_files.append((diff.a_path, diff.b_path))
        else:
            changed_files.append((diff.a_path, diff.a_path))
    return changed_files


def get_target(repo_path: str, file_path: str):
    """
    Run bazel query 'same_pkg_direct_rdeps(<file_path>)' to get the target that owns the file.
    """

    query = f"same_pkg_direct_rdeps({file_path})"
    result = subprocess.run(
        ["bazel", "query", query],
        cwd=repo_path,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print(f"Error running bazel query: {result.stderr}")
        return None
    targets = result.stdout
    return targets


def prerun_commands(prerun: str, repo_path: str):
    if prerun:
        print(f"Running prerun command: {prerun}")
        subprocess.run(prerun, cwd=repo_path, shell=True, check=True)
        print("Prerun command completed.")


def postrun_commands(postrun: str, repo_path: str):
    if postrun:
        print(f"Running postrun command: {postrun}")
        subprocess.run(postrun, cwd=repo_path, shell=True, check=True)
        print("Postrun command completed.")


def main():
    args = parse_args()
    print(f"Collecting commits from repository at: {args.repo_path}")

    os.makedirs(args.result_dir, exist_ok=True)

    repo = Repo(args.repo_path)
    switch_to_commit(repo, args.base_commit)
    commits = get_next_n_commits_from_base(
        repo, args.default_branch, args.base_commit, args.n_commits
    )
    prev_commit = args.base_commit
    for commit in commits:
        if os.path.exists(os.path.join(args.result_dir, f"{commit.hexsha}.json")):
            print(f"Skipping already processed commit: {commit.hexsha}")
            prev_commit = commit.hexsha
            continue

        print(f"Commit: {commit.hexsha} - {commit.message.strip().splitlines()[0]}")
        switch_to_commit(repo, commit.hexsha)

        prerun_commands(args.prerun, args.repo_path)

        changed_files = get_changed_files_between_commits(
            repo, prev_commit, commit.hexsha
        )
        changes = []
        for old_path, new_path in changed_files:
            if any(
                old_path.startswith(excl) or new_path.startswith(excl)
                for excl in args.exclude_paths
            ):
                continue
            target = get_target(args.repo_path, new_path)
            print(f"  Changed file: {old_path} -> {new_path}, Target: {target}")
            changes.append(
                Change(old_path=old_path, new_path=new_path, target=target)
            )

        prev_commit = commit.hexsha
        postrun_commands(args.postrun, args.repo_path)
        result = Result(commit_hash=commit.hexsha, changes=changes)
        with open(os.path.join(args.result_dir, f"{commit.hexsha}.json"), "w") as f:
            f.write(result.model_dump_json(indent=2))


if __name__ == "__main__":
    main()
