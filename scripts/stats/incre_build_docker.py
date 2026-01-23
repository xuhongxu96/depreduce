import os
import subprocess as sp

from scripts.stats.test_incre_build import find_prev_commit, parse_args


def build(
    dir: str,
    build_event_prefix: str,
    extra_args: list[str],
):
    cmd = ["bazel", "build", "--spawn_strategy=local"]
    cmd += [
        f"--build_event_json_file={dir}/{build_event_prefix}.json",
        f"--build_event_text_file={dir}/{build_event_prefix}.txt",
    ]
    cmd += extra_args
    cmd += ["//..."]
    return cmd


def run_in_container(tag: str, cmd: str):
    with open("data/experiment/temp.sh", "w") as f:
        f.write(
            """#!/bin/bash
set -e
"""
            + cmd
            + "\n"
        )

    docker_cmd = [
        "docker",
        "run",
        "--rm",
        "-t",
        "-v",
        "./data/experiment:/mnt",
        tag,
        "/mnt/temp.sh",
    ]
    sp.run(docker_cmd, check=True)


def build_baseline_image(
    repo: str,
    commit: str,
    extra_args: list[str],
    prerun: str,
    postrun: str,
    revert_commit: str = "",
):
    tag = f"depreduce-baseline:{commit}"
    if revert_commit:
        tag += "-revert"

    cmd = [
        "docker",
        "build",
        "--build-arg",
        f"REPO={repo}",
        "--build-arg",
        f"COMMIT={commit}",
        "--build-arg",
        f"REVERT_COMMIT={revert_commit}",
        "--build-arg",
        f"EXTRA_ARGS={' '.join(extra_args)}",
        "--build-arg",
        f"PRERUN={prerun}",
        "--build-arg",
        f"POSTRUN={postrun}",
        # "--no-cache",
        "-t",
        tag,
        "-f",
        "baseline.dockerfile",
        ".",
    ]
    sp.run(cmd, check=True, cwd=os.path.dirname(os.path.abspath(__file__)))
    return tag


def incremental_build(
    pre_git_cmd: str,
    commit: str,
    prerun: str,
    postrun: str,
    extra_args: list[str],
    result_dir: str,
    tag: str,
    label: str,
    iteration: int,
):
    cmd = ""
    cmd += "bazel info\n"
    cmd += pre_git_cmd + "\n"
    if prerun:
        cmd += prerun + "\n"
    cmd += "mkdir -p " + result_dir + "\n"
    cmd += "bazel --version\n"
    cmd += (
        " ".join(
            build(
                extra_args=extra_args,
                dir=result_dir,
                build_event_prefix=f"{commit}-{label}-{iteration}",
            )
        )
        + "\n"
    )
    if postrun:
        cmd += postrun + "\n"

    host_result_dir = result_dir.replace("/app/data/experiment", "/mnt")
    cmd += f"sudo cp -f {result_dir}/* {host_result_dir}/\n"
    cmd += f"sudo chmod -R 777 {host_result_dir}\n"

    run_in_container(tag, cmd)


def test_incre_build(
    repo_name: str,
    commit: str,
    prev_commit: str,
    need_revert: bool,
    base_commit: str,
    result_dir: str,
    prerun: str,
    postrun: str,
    extra_args: list[str],
):
    N_ITERATIONS = 3

    prerun = prerun if prerun else ""
    postrun = postrun if postrun else ""

    after_tag = build_baseline_image(
        repo=repo_name,
        commit=prev_commit,
        extra_args=extra_args,
        prerun=prerun,
        postrun=postrun,
    )

    for i in range(N_ITERATIONS):
        print(f"[After] Running incremental build iteration {i} for commit {commit}...")
        incremental_build(
            pre_git_cmd="git checkout " + commit,
            commit=commit,
            prerun=prerun,
            postrun=postrun,
            extra_args=extra_args,
            result_dir=result_dir,
            tag=after_tag,
            label="after",
            iteration=i,
        )

    before_tag = build_baseline_image(
        repo=repo_name,
        commit=prev_commit,
        extra_args=extra_args,
        prerun=prerun,
        postrun=postrun,
        revert_commit=base_commit if need_revert else "",
    )

    for i in range(N_ITERATIONS):
        print(f"[Before] Running incremental build iteration {i} for commit {commit}...")
        incremental_build(
            pre_git_cmd="git cherry-pick " + commit,
            commit=commit,
            prerun=prerun,
            postrun=postrun,
            extra_args=extra_args,
            result_dir=result_dir,
            tag=before_tag,
            label="before",
            iteration=i,
        )


def main():
    args = parse_args()

    result_dir = args.result_dir.replace("/app/", "./")

    with open(os.path.join(result_dir, "revertible_prev_commits.txt"), "r") as f:
        commits = [line.strip().split(" ") for line in f.readlines()]

    print(f"Testing incremental builds for {len(commits)} commits...")

    for commit, prev_commit, need_revert in commits:
        if commit.strip() == "":
            continue
        test_incre_build(
            os.path.basename(args.repo_path),
            commit,
            prev_commit,
            need_revert == "1",
            args.base_commit,
            os.path.join(args.result_dir, "incre_build"),
            args.prerun,
            args.postrun,
            extra_args=args.extra_build_args,
        )
        break


if __name__ == "__main__":
    main()
