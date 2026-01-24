import os
import json
import numpy as np
from scipy import stats
import pandas as pd
import glob
import matplotlib.pyplot as plt
import seaborn as sns


PROJECTS = ["buildfarm", "copybara", "hirschgarten", "typedb", "angular"]
BASE_DIR = "data/experiment"


def load_revertible_commits(project):
    commits_file = os.path.join(BASE_DIR, project, "stats", "revertible_commits.txt")
    commits_data = {}
    if not os.path.exists(commits_file):
        print(f"Warning: {commits_file} not found.")
        return commits_data

    with open(commits_file, "r") as f:
        for line in f:
            parts = line.strip().split(" ")
            if len(parts) >= 2:
                commit = parts[0]
                try:
                    delta = int(parts[1])
                    commits_data[commit] = delta
                except ValueError:
                    continue
    return commits_data


def get_wall_time(json_path):
    try:
        with open(json_path, "r") as f:
            # The file contains multiple JSON objects, one per line.
            # We need to find the one with buildMetrics.
            for line in f:
                if "buildMetrics" in line:
                    data = json.loads(line)
                    if (
                        "buildMetrics" in data
                        and "timingMetrics" in data["buildMetrics"]
                    ):
                        return int(
                            data["buildMetrics"]["timingMetrics"].get("wallTimeInMs", 0)
                        )
    except Exception as e:
        print(f"Error reading {json_path}: {e}")
    return None


def analyze_project(project):
    print(f"Analyzing project: {project}")
    commits_data = load_revertible_commits(project)

    results = []

    for commit, cost_delta in commits_data.items():
        before_times = []
        after_times = []

        for i in range(5):
            before_json = os.path.join(
                BASE_DIR, project, "stats", "incre_build", f"{commit}-before-{i}.json"
            )
            after_json = os.path.join(
                BASE_DIR, project, "stats", "incre_build", f"{commit}-after-{i}.json"
            )

            if not os.path.exists(before_json) or not os.path.exists(after_json):
                break

            t_before = get_wall_time(before_json)
            t_after = get_wall_time(after_json)

            if t_before is not None:
                before_times.append(t_before)
            if t_after is not None:
                after_times.append(t_after)

        if before_times and after_times:
            median_before = np.median(before_times)
            median_after = np.median(after_times)

            # Improvement: positive means faster (reduced time)
            improvement_ms = median_before - median_after
            improvement_pct = (
                (improvement_ms / median_before) * 100 if median_before > 0 else 0
            )

            # NEW: Calculate mean improvement and CI from independent samples
            mean_before = np.mean(before_times)
            mean_after = np.mean(after_times)
            mean_improvement_ms = mean_before - mean_after

            improvement_error = 0
            n_before = len(before_times)
            n_after = len(after_times)

            if n_before > 1 and n_after > 1:
                var_before = np.var(before_times, ddof=1)
                var_after = np.var(after_times, ddof=1)
                
                # Standard error of the difference of means for independent samples
                std_err_diff = np.sqrt(var_before / n_before + var_after / n_after)
                
                # Degrees of freedom using a conservative estimate: min(n1-1, n2-1)
                dof = min(n_before - 1, n_after - 1)
                
                # t-value for 95% CI
                t_value = stats.t.ppf(0.975, df=dof)
                
                improvement_error = t_value * std_err_diff

            results.append(
                {
                    "commit": commit,
                    "cost_delta": cost_delta,
                    "median_before": median_before,
                    "median_after": median_after,
                    "improvement_ms": improvement_ms,
                    "improvement_pct": improvement_pct,
                    "mean_improvement_ms": mean_improvement_ms,
                    "improvement_error": improvement_error,
                }
            )

    if not results:
        print("  No valid build data found.")
        return

    # Statistical Analysis
    median_befores = [r["median_before"] for r in results]
    median_afters = [r["median_after"] for r in results]
    improvements_ms = [r["improvement_ms"] for r in results]
    improvements_pct = [r["improvement_pct"] for r in results]
    cost_deltas = [r["cost_delta"] for r in results]

    avg_improvement_ms = np.mean(improvements_ms)
    median_improvement_ms = np.median(improvements_ms)
    avg_improvement_pct = np.mean(improvements_pct)
    median_improvement_pct = np.median(improvements_pct)

    # Confidence Interval for Improvement (Mean)
    if len(improvements_ms) > 1:
        ci = stats.t.interval(
            0.95,
            len(improvements_ms) - 1,
            loc=np.mean(improvements_ms),
            scale=stats.sem(improvements_ms),
        )
    else:
        ci = (improvements_ms[0], improvements_ms[0]) if improvements_ms else (0, 0)

    print(f"  Number of analyzed commits: {len(results)}")
    print(f"  Median Build Time Before: {np.median(median_befores):.2f} ms")
    print(f"  Median Build Time After: {np.median(median_afters):.2f} ms")
    print(
        f"  Average Improvement: {avg_improvement_ms:.2f} ms ({avg_improvement_pct:.2f}%)"
    )
    print(
        f"  Median Improvement: {median_improvement_ms:.2f} ms ({median_improvement_pct:.2f}%)"
    )
    print(f"  95% CI for Mean Improvement (ms): {ci}")

    # Correlation
    correlation_text = "  Not enough data for correlation analysis."
    correlation, p_value = None, None
    if len(results) > 1:
        # Correlation is calculated on the mean improvement now
        mean_improvements = [r["mean_improvement_ms"] for r in results]
        correlation, p_value = stats.pearsonr(cost_deltas, mean_improvements)
        correlation_text = f"  Correlation between Rebuild Cost Delta and Time Improvement (ms): {correlation:.4f} (p-value: {p_value:.4f})"
        print(correlation_text)
    else:
        print(correlation_text)

    print("-" * 40)

    return {
        "project": project,
        "count": len(results),
        "median_before": np.median(median_befores),
        "median_after": np.median(median_afters),
        "avg_improvement_ms": avg_improvement_ms,
        "avg_improvement_pct": avg_improvement_pct,
        "median_improvement_ms": median_improvement_ms,
        "median_improvement_pct": median_improvement_pct,
        "ci": ci,
        "correlation": correlation,
        "p_value": p_value,
        "raw_results": results,
    }


def print_summary(stats_data):
    if not stats_data:
        return

    print("\n" + "=" * 60)
    print(f"AUTOMATED INSIGHTS SUMMARY FOR: {stats_data['project'].upper()}")
    print("=" * 60)

    # 1. Analyzed Commits
    print(f"* **Analyzed Commits:** {stats_data['count']}")

    # 2. Build Time
    print(
        f"* **Build Time:** The median build time was {stats_data['median_before']:.2f} ms (Before) vs {stats_data['median_after']:.2f} ms (After)."
    )

    # 3. Improvement
    print("* **Improvement:**")
    print(
        f"    * **Average Improvement:** {stats_data['avg_improvement_ms']:.2f} ms ({stats_data['avg_improvement_pct']:.2f}%)."
    )
    print(
        f"    * **Median Improvement:** {stats_data['median_improvement_ms']:.2f} ms ({stats_data['median_improvement_pct']:.2f}%)."
    )

    # 4. Statistical Significance
    ci_lower, ci_upper = stats_data["ci"]
    is_significant = (ci_lower > 0) or (ci_upper < 0)  # Simple check if 0 is in CI
    print(
        f"* **Statistical Significance:** The 95% confidence interval for the mean improvement is ({ci_lower:.2f} ms, {ci_upper:.2f} ms)."
    )
    if is_significant:
        if stats_data["avg_improvement_ms"] > 0:
            print(
                "    * **Conclusion:** The improvement is statistically significant and positive."
            )
        else:
            print(
                "    * **Conclusion:** There is a statistically significant regression (slowdown)."
            )
    else:
        print(
            "    * **Conclusion:** The interval includes zero, so the result is not statistically significant (on average)."
        )

    # 5. Correlation
    if stats_data["correlation"] is not None and stats_data["p_value"] is not None:
        corr = stats_data["correlation"]
        p = stats_data["p_value"]
        print(
            f"* **Correlation:** Pearson correlation between 'Rebuild Cost Delta' and 'Time Improvement' is {corr:.4f} (p-value: {p:.4f})."
        )

        if p < 0.05:
            if corr > 0.5:
                print(
                    "    * **Insight:** Strong positive correlation. Higher structural cost reductions consistently translate to better build time savings."
                )
            elif corr > 0:
                print(
                    "    * **Insight:** Moderate/Weak positive correlation. Structural improvements tend to help, but build noise may be a factor."
                )
            elif corr < 0:
                print(
                    "    * **Insight:** Negative correlation. This is unexpected; larger structural changes might be causing regressions or unrelated issues."
                )
        else:
            print(
                "    * **Insight:** No statistically significant correlation found (p >= 0.05). Build time changes might be dominated by noise or other factors."
            )
    else:
        print("* **Correlation:** Not enough data points to calculate correlation.")


def plot_all_projects(all_stats):
    """
    Generate and save plots for all projects.
    """
    print("\n" + "=" * 60)
    print("Generating plots...")
    
    # Filter projects with less than or equal to 3 data points for plotting
    plottable_stats = [s for s in all_stats if s["count"] > 3]
    
    plotted_projects = {s['project'] for s in plottable_stats}
    skipped_projects = {s['project'] for s in all_stats} - plotted_projects
    
    if skipped_projects:
        print(f"Skipping projects with <= 3 data points from plot: {', '.join(skipped_projects)}")

    print("=" * 60)

    if not plottable_stats:
        print("No projects with sufficient data (> 3 data points) to plot.")
        return

    # Create a directory for plots
    output_dir = "plots"
    os.makedirs(output_dir, exist_ok=True)

    # Plot 1: Scatter plot of Cost Delta vs. Time Improvement (combined)
    plot_scatter_combined(plottable_stats, output_dir)

    print(f"Plots saved to '{output_dir}' directory.")


def plot_scatter_combined(all_stats, output_dir):
    """
    Generates a scatter plot of Cost Delta vs. Time Improvement for all projects,
    with error bars for each point representing the confidence interval.
    """
    sns.set_theme(style="whitegrid")
    
    # Consolidate data
    all_results = []
    for s in all_stats:
        for res in s.get("raw_results", []):
            res["project"] = s["project"]
            all_results.append(res)
    
    if not all_results:
        print("No data to plot for scatter plot.")
        return

    df = pd.DataFrame(all_results)

    # Create plot
    fig, ax = plt.subplots(figsize=(14, 9))

    projects = df["project"].unique()
    palette = sns.color_palette("viridis", len(projects))
    
    for i, project in enumerate(projects):
        project_df = df[df["project"] == project]
        
        # Plot error bars
        ax.errorbar(
            x=project_df["cost_delta"],
            y=project_df["mean_improvement_ms"],
            yerr=project_df["improvement_error"],
            fmt='o',  # 'o' for points, no connecting line
            color=palette[i],
            label=project,
            capsize=3,
            elinewidth=1,
            alpha=0.6
        )
        
        # Plot regression line
        sns.regplot(
            data=project_df,
            x="cost_delta",
            y="mean_improvement_ms",
            ax=ax,
            color=palette[i],
            scatter=False, # Don't plot the scatter points again
        )

    ax.set_xlabel("Rebuild Cost Delta (Number of Nodes)")
    ax.set_ylabel("Build Time Improvement (ms)")
    ax.set_title("Correlation between Rebuild Cost Delta and Build Time Improvement (with 95% CI for each point)")
    ax.legend()
    ax.axhline(0, color='grey', linestyle='--')

    plt.savefig(os.path.join(output_dir, "scatter_cost_vs_improvement_combined.png"), bbox_inches="tight")
    plt.close(fig)


def main():
    all_stats = []
    for project in PROJECTS:
        s = analyze_project(project)
        if s:
            all_stats.append(s)

    # Print individual summaries
    for s in all_stats:
        print_summary(s)

    # Generate and save plots
    if all_stats:
        plot_all_projects(all_stats)

    # Print Global Summary
    print("\n" + "=" * 60)
    print("GLOBAL OBSERVATIONS")
    print("=" * 60)

    if not all_stats:
        print("No data to analyze.")
        return

    # Filter for global observations summary
    valid_stats = [s for s in all_stats if s["count"] > 3]
    if not valid_stats:
        print("No projects with sufficient data for global observations.")
        return
        
    best_project = max(valid_stats, key=lambda x: x["count"])
    print(
        f"Across all projects, **{best_project['project'].capitalize()}** provides the most robust dataset with {best_project['count']} commits."
    )

    if (
        best_project["correlation"] is not None
        and best_project["p_value"] < 0.05
        and best_project["correlation"] > 0.3
    ):
        print(
            f"For {best_project['project'].capitalize()}, we observe a meaningful correlation ({best_project['correlation']:.2f}) between structural 'Rebuild Cost Delta' and actual time savings."
        )

    print(
        "In projects with very few data points or extremely long build times, build noise often overshadows the benefits of dependency reduction."
    )


if __name__ == "__main__":
    main()
