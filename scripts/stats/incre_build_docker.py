import os
import subprocess as sp

from scripts.stats.test_incre_build import find_prev_commit, parse_args


def build(
    extra_args: list[str],
    dir: str | None = None,
    build_event_prefix: str | None = None,
    use_local_spawn: bool = True,
):
    cmd = ["bazel", "build"]
    if use_local_spawn:
        cmd += ["--spawn_strategy=local"]
    if dir and build_event_prefix:
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
    repo_name: str,
    pre_git_cmd: str,
    commit: str,
    prerun: str,
    postrun: str,
    extra_args: list[str],
    result_dir: str,
    tag: str,
    label: str,
    iteration: int,
    build_baseline: bool = False,
    prev_commit: str = "",
    revert_commit: str = "",
):
    cmd = ""
    if build_baseline:
        cmd += f"cd /app/{repo_name}\n"
        cmd += "git checkout " + prev_commit + "\n"
        if revert_commit:
            cmd += "git revert --no-edit " + revert_commit + "\n"
        cmd += "git submodule update\n"
        if prerun:
            cmd += prerun + "\n"
        cmd += " ".join(build(extra_args=extra_args, use_local_spawn=False)) + "\n"
        if postrun:
            cmd += postrun + "\n"

    cmd += "bazel info\n"
    cmd += pre_git_cmd + "\n"
    cmd += "git submodule update\n"
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
                use_local_spawn=not build_baseline,
            )
        )
        + "\n"
    )
    if postrun:
        cmd += postrun + "\n"

    host_result_dir = result_dir.replace("/app/data/experiment", "/mnt")
    cmd += f"sudo mkdir -p {host_result_dir}\n"
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
    cache_baseline: bool,
):
    N_ITERATIONS = 5

    prerun = prerun if prerun else ""
    postrun = postrun if postrun else ""
    host_result_dir = result_dir.replace("/app/data", "./data")

    if cache_baseline:
        after_tag = build_baseline_image(
            repo=repo_name,
            commit=prev_commit,
            extra_args=extra_args,
            prerun=prerun,
            postrun=postrun,
        )

    for i in range(N_ITERATIONS):
        print(os.path.join(host_result_dir, f"{commit}-after-{i}.json"))
        if os.path.exists(os.path.join(host_result_dir, f"{commit}-after-{i}.json")):
            print(f"Skipping {commit} after iteration {i} as results already exist.")
            continue

        print(f"[After] Running incremental build iteration {i} for commit {commit}...")
        incremental_build(
            repo_name=repo_name,
            pre_git_cmd="git checkout " + commit,
            commit=commit,
            prerun=prerun,
            postrun=postrun,
            extra_args=extra_args,
            result_dir=result_dir,
            tag=after_tag if cache_baseline else "depreduce:latest",
            label="after",
            iteration=i,
            build_baseline=not cache_baseline,
            prev_commit=prev_commit,
        )

    if cache_baseline:
        before_tag = build_baseline_image(
            repo=repo_name,
            commit=prev_commit,
            extra_args=extra_args,
            prerun=prerun,
            postrun=postrun,
            revert_commit=base_commit if need_revert else "",
        )

    for i in range(N_ITERATIONS):
        if os.path.exists(os.path.join(host_result_dir, f"{commit}-before-{i}.json")):
            print(f"Skipping {commit} after iteration {i} as results already exist.")
            continue
        print(
            f"[Before] Running incremental build iteration {i} for commit {commit}..."
        )
        incremental_build(
            repo_name=repo_name,
            pre_git_cmd="git cherry-pick " + commit,
            commit=commit,
            prerun=prerun,
            postrun=postrun,
            extra_args=extra_args,
            result_dir=result_dir,
            tag=before_tag if cache_baseline else "depreduce:latest",
            label="before",
            iteration=i,
            build_baseline=not cache_baseline,
            prev_commit=prev_commit,
            revert_commit=base_commit if need_revert else "",
        )


def main():
    args = parse_args()

    result_dir = args.result_dir.replace("/app/", "./")

    with open(os.path.join(result_dir, "revertible_prev_commits.txt"), "r") as f:
        commits = [line.strip().split(" ") for line in f.readlines()]

    print(f"Testing incremental builds for {len(commits)} commits...")
    print(f"{args.baseline_docker_cache=}")

    for commit, prev_commit, need_revert in commits:
        try:
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
                cache_baseline=args.baseline_docker_cache,
            )
        except Exception as e:
            import time

            print(e)

            with open(os.path.join(result_dir, "incre_build_errors.txt"), "a") as ef:
                ef.write(f"[{time.time()}] Error testing commit {commit}: {e}\n")


if __name__ == "__main__":
    main()
