import argparse
import os
from pathlib import Path
import shlex
import subprocess as sp
import sys
import time

sys.path.append(str(Path(__file__).resolve().parents[2]))

from scripts.stats.collect_commits import RunArgs
from scripts.stats.incre_build_docker import build


def parse_args():
    argparser = argparse.ArgumentParser()
    argparser.add_argument(
        "-c",
        "--config",
        type=str,
        required=True,
        help="Path to the configuration file",
    )
    argparser.add_argument(
        "--iterations",
        type=int,
        default=5,
        help="Number of clean build iterations to run for each commit",
    )
    argparser.add_argument(
        "--image",
        type=str,
        default="depreduce:latest",
        help="Docker image to use for the isolated build",
    )
    argparser.add_argument(
        "--force",
        action="store_true",
        help="Rerun builds even when both build event outputs already exist",
    )
    args = argparser.parse_args()

    with open(args.config, "r") as f:
        config_data = f.read()

    return args, RunArgs.model_validate_json(config_data)


def run_in_container(image: str, cmd: str):
    script_path = "data/experiment/temp.sh"
    with open(script_path, "w") as f:
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
        image,
        "/mnt/temp.sh",
    ]
    sp.run(docker_cmd, check=True)


def host_result_dir(result_dir: str):
    return result_dir.replace("/app/data", "./data")


def container_mounted_result_dir(result_dir: str):
    return result_dir.replace("/app/data/experiment", "/mnt")


def outputs_exist(result_dir: str, prefix: str):
    host_dir = host_result_dir(result_dir)
    return os.path.exists(os.path.join(host_dir, f"{prefix}.json")) and os.path.exists(
        os.path.join(host_dir, f"{prefix}.txt")
    )


def quote_cmd(cmd: str):
    return shlex.quote(cmd)


def clean_build(
    image: str,
    repo_path: str,
    commit: str,
    result_dir: str,
    prefix: str,
    prerun: str,
    postrun: str,
    extra_args: list[str],
):
    build_cmd = " ".join(
        shlex.quote(arg)
        for arg in build(
            extra_args=extra_args,
            dir=result_dir,
            build_event_prefix=prefix,
            use_local_spawn=False,
        )
    )

    cmd = ""
    cmd += f"cd {quote_cmd(repo_path)}\n"
    cmd += f"git checkout {quote_cmd(commit)}\n"
    cmd += "git submodule update\n"
    if prerun:
        cmd += prerun + "\n"
    cmd += f"mkdir -p {quote_cmd(result_dir)}\n"
    cmd += "bazel --version\n"
    cmd += "bazel clean --expunge\n"
    cmd += build_cmd + "\n"
    if postrun:
        cmd += postrun + "\n"

    mounted_result_dir = container_mounted_result_dir(result_dir)
    cmd += f"sudo mkdir -p {quote_cmd(mounted_result_dir)}\n"
    cmd += f"sudo cp -f {quote_cmd(result_dir)}/* {quote_cmd(mounted_result_dir)}/\n"
    cmd += f"sudo chmod -R 777 {quote_cmd(mounted_result_dir)}\n"

    run_in_container(image, cmd)


def log_error(result_dir: str, repo_name: str, label: str, commit: str, iteration: int, error):
    os.makedirs(host_result_dir(result_dir), exist_ok=True)
    error_path = os.path.join(host_result_dir(result_dir), "clean_build_errors.txt")
    with open(error_path, "a") as ef:
        ef.write(
            f"[{time.time()}] Error testing {repo_name} {label} "
            f"commit {commit} iteration {iteration}: {error}\n"
        )


def main():
    cli_args, config = parse_args()

    if cli_args.iterations < 1:
        raise ValueError("--iterations must be at least 1")

    result_dir = os.path.join(config.result_dir, "clean_build")
    os.makedirs(host_result_dir(result_dir), exist_ok=True)

    repo_name = os.path.basename(config.repo_path)
    prerun = config.prerun if config.prerun else ""
    postrun = config.postrun if config.postrun else ""
    before_commit = f"{config.base_commit}^"
    runs = [
        ("before", before_commit),
        ("after", config.base_commit),
    ]

    print(
        f"Testing clean builds for {repo_name}: "
        f"before={before_commit}, after={config.base_commit}"
    )

    for label, commit in runs:
        for iteration in range(cli_args.iterations):
            prefix = f"{config.base_commit}-{label}-{iteration}"
            if not cli_args.force and outputs_exist(result_dir, prefix):
                print(
                    f"Skipping {repo_name} {label} iteration {iteration} "
                    "as results already exist."
                )
                continue

            print(
                f"[{label.capitalize()}] Running clean build iteration {iteration} "
                f"for {repo_name} at {commit}..."
            )
            try:
                clean_build(
                    image=cli_args.image,
                    repo_path=config.repo_path,
                    commit=commit,
                    result_dir=result_dir,
                    prefix=prefix,
                    prerun=prerun,
                    postrun=postrun,
                    extra_args=config.extra_build_args,
                )
            except Exception as e:
                print(e)
                log_error(result_dir, repo_name, label, commit, iteration, e)


if __name__ == "__main__":
    main()
