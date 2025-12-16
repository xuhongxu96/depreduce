import json
import os
import glob
import argparse

from pydantic import BaseModel
from scripts.stats.collect_commits import Result


class RunArgs(BaseModel):
    result_dir: str


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
    for path in glob.glob(os.path.join(args.result_dir, "*.json")):
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


if __name__ == "__main__":
    main()
