import json
import argparse
from pydantic import BaseModel


class RunArgs(BaseModel):
    rebuild_sets: list[str]
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


def load_rebuild_set(file_path: str) -> dict[str, list[str]]:
    with open(file_path, "r") as f:
        data = json.load(f)
    rebuild_set = {}
    for item in data["rebuild_set"]:
        key = item[0]
        values = item[1]
        rebuild_set[key] = values
    return rebuild_set


def get_changed_rebuild_sets(
    set1: dict[str, list[str]], set2: dict[str, list[str]]
) -> dict[str, dict[str, list[str]]]:
    changed = {}
    all_keys = set(set1.keys()).union(set(set2.keys()))
    for key in all_keys:
        values1 = set1.get(key, [])
        values2 = set2.get(key, [])
        if sorted(values1) != sorted(values2):
            changed[key] = {
                "set1": values1,
                "set2": values2,
                "len1": len(values1),
                "len2": len(values2),
            }
    return changed


def main():
    args = parse_args()
    rebuild_sets = args.rebuild_sets
    if len(rebuild_sets) < 2:
        print("At least two rebuild sets are required for comparison.")
        return
    set1 = load_rebuild_set(rebuild_sets[0])
    set2 = load_rebuild_set(rebuild_sets[1])
    changed_sets = get_changed_rebuild_sets(set1, set2)
    changed_pkg = []
    print(f"Total changed rebuild sets: {len(changed_sets)}")
    for key, changes in changed_sets.items():
        if changes['len1'] <= changes['len2']:
            print(f"Package: {key}")
            print(f"  Set1 (len={changes['len1']})")
            print(f"  Set2 (len={changes['len2']})")
            print("Skipping smaller or equal set changes.")
            continue
        changed_pkg.append((key, changes['len1'], changes['len2']))

    with open(f"{args.result_dir}/changed_rebuild_sets.json", "w") as f:
        json.dump(changed_pkg, f, indent=2)


if __name__ == "__main__":
    main()
