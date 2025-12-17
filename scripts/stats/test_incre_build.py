import os
import subprocess as sp

from git import Repo

from scripts.stats.collect_commits import (
    RunArgs,
    Result,
    parse_args,
    get_next_n_commits_from_base,
    switch_to_commit,
    prerun_commands,
    postrun_commands,
)


def find_prev_commit(repo: Repo, commit: str) -> str | None:
    git_commit = repo.commit(commit)
    parents = git_commit.parents
    if len(parents) == 0:
        return None
    return parents[0].hexsha


def clean_build(cwd: str):
    sp.run(["bazel", "clean", "--expunge"], check=True, cwd=cwd)


def build(
    cwd: str,
    extra_args: list[str] = [],
    build_event_dir: str | None = None,
    build_event_prefix: str | None = None,
):
    cmd = ["bazel", "build"]
    if build_event_dir is not None and build_event_prefix is not None:
        cmd += [
            f"--build_event_json_file={build_event_dir}/{build_event_prefix}.json",
            f"--build_event_text_file={build_event_dir}/{build_event_prefix}.txt",
        ]
    cmd += extra_args
    cmd += ["//..."]

    sp.run(cmd, check=True, cwd=cwd)


def revert_commit(repo: Repo, commit: str):
    repo.git.execute(f"git revert {commit}".split())


def cherrypick_commit(repo: Repo, commit: str):
    repo.git.execute(f"git cherry-pick {commit}".split())


def test_incre_build(
    repo: Repo,
    commit: str,
    base_commit: str,
    result_dir: str,
    prerun: str,
    postrun: str,
    extra_args: list[str],
):
    prev_commit = find_prev_commit(repo, commit)
    need_revert = True
    if prev_commit == base_commit:
        prev_commit = find_prev_commit(repo, base_commit)
        need_revert = False

    postrun_commands(postrun, repo.working_tree_dir)

    switch_to_commit(repo, prev_commit)
    clean_build(repo.working_tree_dir)

    prerun_commands(prerun, repo.working_tree_dir)

    build(repo.working_tree_dir, extra_args=extra_args)

    postrun_commands(postrun, repo.working_tree_dir)

    switch_to_commit(repo, commit)

    prerun_commands(prerun, repo.working_tree_dir)

    build(
        repo.working_tree_dir,
        extra_args=extra_args,
        build_event_dir=result_dir,
        build_event_prefix=f"{commit}-after",
    )

    postrun_commands(postrun, repo.working_tree_dir)

    switch_to_commit(repo, prev_commit)
    if need_revert:
        revert_commit(repo, commit)
    clean_build(repo.working_tree_dir)

    prerun_commands(prerun, repo.working_tree_dir)

    build(repo.working_tree_dir, extra_args=extra_args)

    postrun_commands(postrun, repo.working_tree_dir)

    cherrypick_commit(repo, commit)

    prerun_commands(prerun, repo.working_tree_dir)

    build(
        repo.working_tree_dir,
        extra_args=extra_args,
        build_event_dir=result_dir,
        build_event_prefix=f"{commit}-before",
    )
    postrun_commands(postrun, repo.working_tree_dir)


def main():
    args = parse_args()

    repo = Repo(args.repo_path)

    with open(os.path.join(args.result_dir, "revertible_commits.txt"), "r") as f:
        commits = [line.strip() for line in f.readlines()]

    print(f"Testing incremental builds for {len(commits)} commits...")

    for commit in commits:
        test_incre_build(
            repo,
            commit,
            args.base_commit,
            os.path.join(args.result_dir, "incre_build"),
            args.prerun,
            args.postrun,
            extra_args=args.extra_build_args,
        )


if __name__ == "__main__":
    main()
