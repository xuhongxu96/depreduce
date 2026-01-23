# Scripts for Performance Statistics and Analysis

This directory contains a suite of Python scripts designed to collect, process, and analyze build performance data for the `depreduce` project. The scripts are meant to be run in a sequence to measure the impact of build optimizations on incremental build times.

## Workflow Overview

The general workflow is as follows:

1.  **Data Collection**: Identify a series of recent commits and the build targets they affect.
2.  **Filtering**: From those commits, identify which ones are "optimized" (i.e., affect a part of the build system that has been improved) and are "revertible" for A/B testing.
3.  **Experiment Execution**: For each of these commits, run a controlled incremental build experiment to measure the build time "before" (with the optimization reverted) and "after" (with the optimization applied).
4.  **Analysis**: Process the raw build data to perform statistical analysis and generate plots to visualize the performance improvements.

## Script Descriptions

Here is a breakdown of each script and its purpose in the workflow:

### `collect_commits.py`

-   **Purpose**: The first step in the data collection pipeline. It iterates through a repository's commit history from a specified `base_commit`. For each commit, it finds the changed files and their associated `bazel` build targets.
-   **Input**: A JSON configuration file specifying the repository path, commit range, etc.
-   **Output**: A series of JSON files (one per commit) in the `result_dir`, detailing the changes.

### `compare_rebuild_sets.py`

-   **Purpose**: Compares two "rebuild set" JSON files to identify which build targets have had their rebuild dependency sets changed (and presumably, reduced).
-   **Input**: Paths to two rebuild set JSON files.
-   **Output**: A `changed_rebuild_sets.json` file, listing the optimized targets.

### `check_coverage.py`

-   **Purpose**: Filters the commits gathered by `collect_commits.py`. It uses the `changed_rebuild_sets.json` to find commits that affect optimized targets. It then checks if these commits can be cleanly reverted with `git revert` to ensure they are suitable for A/B testing.
-   **Input**: The output directories from `collect_commits.py` and `compare_rebuild_sets.py`.
-   **Output**: `revertible_commits.txt`, which lists the commits that can be used for performance experiments and includes a "cost delta" metric for each.

### `collect_prev_commits.py`

-   **Purpose**: A helper script that finds the parent commit for each commit listed in `revertible_commits.txt`. This is needed to set up the "before" and "after" states for testing.
-   **Input**: `revertible_commits.txt`.
-   **Output**: `revertible_prev_commits.txt`, containing pairs of `(commit, parent_commit)`.

### `incre_build_docker.py`

-   **Purpose**: Orchestrates the main performance experiment. For each commit pair from `revertible_prev_commits.txt`, it builds a "baseline" Docker image and runs the "before" and "after" incremental builds inside the container. It runs each build multiple times to gather multiple data points.
-   **Input**: `revertible_prev_commits.txt`.
-   **Output**: A series of build event JSON files (e.g., `<commit>-before-0.json`, `<commit>-after-0.json`, etc.) containing detailed build metrics.

### `test_incre_build.py`

-   **Purpose**: A local alternative to `incre_build_docker.py`. It performs the same incremental build tests but on the local machine instead of within a Docker container.
-   **Input**: `revertible_commits.txt`.
-   **Output**: The same build event JSON files as the Docker version.

### `collect_incre_build.py`

-   **Purpose**: A simple parsing script to get a quick summary of the results. It reads all the build event JSON files and prints key metrics like actions executed, CPU time, and wall time for the "before" and "after" runs.
-   **Input**: The directory of build event JSON files.
-   **Output**: A summary printed to the console.

### `analyze_builds.py`

-   **Purpose**: The final analysis and visualization script. It processes the `revertible_commits.txt` and the build event JSON files to perform a detailed statistical analysis. It calculates the mean improvement, confidence intervals, and the correlation between the "cost delta" and the actual build time improvement.
-   **Input**: `revertible_commits.txt` and the directory of build event JSON files.
-   **Output**: A detailed summary printed to the console and a scatter plot (`scatter_cost_vs_improvement_combined.png`) saved to the `plots/` directory.

### `Dockerfile` & `baseline.dockerfile`

-   **Purpose**: These files define the Docker environments used for the experiments.
-   **`Dockerfile`**: Defines the main environment with all necessary tools and cloned repositories.
-   **`baseline.dockerfile`**: Used by `incre_build_docker.py` to create specific, clean build environments for a single repository at a specific commit.
