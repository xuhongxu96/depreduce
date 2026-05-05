#!/usr/bin/env python3
"""Analyze clean build times before/after and produce publication-quality plots.

Usage: scripts/stats/analyze_clean_build.py --data-root data/experiment --min-runs 5
"""

import argparse
import json
import os
import glob
import re
from collections import defaultdict
from datetime import datetime

import numpy as np
import pandas as pd
import matplotlib as mpl
import matplotlib.pyplot as plt
from matplotlib.ticker import MaxNLocator
from matplotlib.font_manager import FontProperties
import seaborn as sns


def parse_args():
    p = argparse.ArgumentParser()
    p.add_argument(
        "--data-root", default="data/experiment", help="root experiments folder"
    )
    p.add_argument(
        "--min-runs",
        type=int,
        default=5,
        help="minimum runs before/after to include project",
    )
    p.add_argument(
        "--out-dir", default="plots", help="output folder for plots and summary"
    )
    p.add_argument(
        "--bootstrap", type=int, default=2000, help="bootstrap samples for median CI"
    )
    return p.parse_args()


def safe_int(v):
    try:
        return int(v)
    except Exception:
        return None


def ms_from_iso(ts):
    try:
        dt = datetime.fromisoformat(ts.replace("Z", "+00:00"))
        return int(dt.timestamp() * 1000)
    except Exception:
        return None


def extract_build_time_ms(path):
    """Stream file and try to extract start/finish times (ms).

    Strategy:
    - read file line-by-line as JSON records (JSONL produced by Bazel build events)
    - look for 'started' -> startTimeMillis and 'finished' -> finishTimeMillis
    - fallback: if buildMetrics.actionSummary.actionData present, use min(firstStartedMs) and max(lastEndedMs)
    - return duration in milliseconds or None
    """
    start = None
    finish = None
    min_action_start = None
    max_action_end = None
    try:
        with open(path, "r", encoding="utf-8") as fh:
            for ln in fh:
                ln = ln.strip()
                if not ln:
                    continue
                try:
                    obj = json.loads(ln)
                except Exception:
                    continue
                # top-level started
                if "started" in obj and isinstance(obj["started"], dict):
                    v = obj["started"].get("startTimeMillis")
                    si = safe_int(v)
                    if si:
                        start = si if start is None else min(start, si)
                # top-level finished
                if "finished" in obj and isinstance(obj["finished"], dict):
                    v = obj["finished"].get("finishTimeMillis")
                    fi = safe_int(v)
                    if fi:
                        finish = fi if finish is None else max(finish, fi)
                    else:
                        # sometimes finishTime is iso string
                        iso = obj["finished"].get("finishTime")
                        if iso:
                            fi2 = ms_from_iso(iso)
                            if fi2:
                                finish = fi2 if finish is None else max(finish, fi2)
                # nested buildMetrics -> actionSummary
                if "buildMetrics" in obj and isinstance(obj["buildMetrics"], dict):
                    actionSummary = obj["buildMetrics"].get("actionSummary") or {}
                    actionData = actionSummary.get("actionData") or []
                    for a in actionData:
                        fs = safe_int(a.get("firstStartedMs"))
                        le = safe_int(a.get("lastEndedMs"))
                        if fs:
                            min_action_start = (
                                fs
                                if min_action_start is None
                                else min(min_action_start, fs)
                            )
                        if le:
                            max_action_end = (
                                le
                                if max_action_end is None
                                else max(max_action_end, le)
                            )
    except Exception:
        return None

    if start is not None and finish is not None and finish >= start:
        return finish - start
    if (
        min_action_start is not None
        and max_action_end is not None
        and max_action_end >= min_action_start
    ):
        return max_action_end - min_action_start
    return None


def bootstrap_median_ci(arr, n_boot=2000, rng=None):
    arr = np.asarray(arr)
    if arr.size == 0:
        return (None, None)
    if arr.size == 1:
        return (arr[0], arr[0])
    rng = rng or np.random.default_rng(12345)
    meds = []
    for _ in range(n_boot):
        sample = rng.choice(arr, size=arr.size, replace=True)
        meds.append(np.median(sample))
    lo, hi = np.percentile(meds, [2.5, 97.5])
    return (float(lo), float(hi))


def summarize_project(dirpath, min_runs=5, bootstrap=2000):
    files = sorted(glob.glob(os.path.join(dirpath, "*.json")))
    before = []
    after = []
    # dirpath: data/experiment/<project>/stats/clean_build
    # want project name = basename of data/experiment/<project>
    name = os.path.basename(os.path.dirname(os.path.dirname(dirpath)))
    # classify by filename containing '-before-' or '-after-'
    for f in files:
        base = os.path.basename(f)
        if "-before-" in base:
            v = extract_build_time_ms(f)
            if v is not None:
                before.append(v)
        elif "-after-" in base:
            v = extract_build_time_ms(f)
            if v is not None:
                after.append(v)

    if len(before) < min_runs or len(after) < min_runs:
        print(
            f"Skipping {name} as it has insufficient runs (before={len(before)}, after={len(after)})"
        )
        return None

    b_med = float(np.median(before))
    a_med = float(np.median(after))
    b_lo, b_hi = bootstrap_median_ci(before, n_boot=bootstrap)
    a_lo, a_hi = bootstrap_median_ci(after, n_boot=bootstrap)

    improvement_ms = b_med - a_med
    improvement_pct = (improvement_ms / b_med * 100.0) if b_med else None

    return {
        "project": name,
        "before_median_ms": b_med,
        "before_ci_lo_ms": b_lo,
        "before_ci_hi_ms": b_hi,
        "after_median_ms": a_med,
        "after_ci_lo_ms": a_lo,
        "after_ci_hi_ms": a_hi,
        "improvement_ms": improvement_ms,
        "improvement_pct": improvement_pct,
        "runs_before": len(before),
        "runs_after": len(after),
    }


def find_candidate_dirs(data_root):
    pattern = os.path.join(data_root, "*", "stats", "clean_build")
    return sorted([p for p in glob.glob(pattern) if os.path.isdir(p)])


def make_plot(df, out_path_png, out_path_pdf, figsize=(2.5, 1.2), dpi=300):
    legend_font_prop = FontProperties(family="Times New Roman", size=5)
    font_prop = FontProperties(family="Times New Roman", size=6)
    mpl.rcParams["font.family"] = font_prop.get_name()

    sns.set_style("whitegrid")

    projects = df["project"].tolist()
    x = np.arange(len(projects))
    width = 0.2

    fig, ax = plt.subplots(figsize=figsize)

    before_vals = df["before_median_ms"].values
    after_vals = df["after_median_ms"].values
    before_err_low = df["before_median_ms"] - df["before_ci_lo_ms"]
    before_err_high = df["before_ci_hi_ms"] - df["before_median_ms"]
    after_err_low = df["after_median_ms"] - df["after_ci_lo_ms"]
    after_err_high = df["after_ci_hi_ms"] - df["after_median_ms"]

    error_style = {"elinewidth": 0.5, "capthick": 0.5}
    ax.bar(
        x - width / 2,
        before_vals,
        width,
        label="Before",
        color="#4C72B0",
        yerr=[before_err_low, before_err_high],
        capsize=1,
        error_kw=error_style,
    )
    ax.bar(
        x + width / 2,
        after_vals,
        width,
        label="After",
        color="#DD8452",
        yerr=[after_err_low, after_err_high],
        capsize=1,
        error_kw=error_style,
    )

    ax.set_xticks(x)
    ax.set_xticklabels(projects, fontproperties=font_prop)
    ax.set_ylabel("Clean build time (ms)", fontproperties=font_prop)
    ax.yaxis.set_major_locator(MaxNLocator(nbins=6))
    ax.set_axisbelow(True)
    ax.grid(axis="y", linewidth=0.3, color="#D9D9D9", alpha=0.9)
    ax.grid(axis="x", linewidth=0.3, color="#D9D9D9", alpha=0.9)
    ax.legend(
        frameon=True,
        facecolor="white",
        edgecolor="none",
        framealpha=1.0,
        ncol=1,
        loc="upper left",
        borderaxespad=0.2,
        handlelength=1.5,
        columnspacing=0.4,
        prop=legend_font_prop,
    )
    for tick in ax.get_yticklabels():
        tick.set_fontproperties(font_prop)

    # annotate improvement percent
    for i, row in df.iterrows():
        pct = row["improvement_pct"]
        txt = f"{pct:.1f}%" if pct is not None else "n/a"
        y = max(row["before_median_ms"], row["after_median_ms"])
        ax.text(
            i + 0.1, y * 1.04, txt, ha="center", fontsize=5, fontproperties=font_prop
        )

    sns.despine(ax=ax)
    fig.tight_layout(pad=0.1)
    os.makedirs(os.path.dirname(out_path_png), exist_ok=True)
    fig.savefig(out_path_pdf, bbox_inches="tight", pad_inches=0.03)
    plt.close(fig)


def main():
    args = parse_args()
    dirs = find_candidate_dirs(args.data_root)
    results = []
    for d in dirs:
        s = summarize_project(d, min_runs=args.min_runs, bootstrap=args.bootstrap)
        if s:
            results.append(s)

    if not results:
        print(
            "No qualifying projects found (need >= min-runs for both before and after)."
        )
        return 1

    df = pd.DataFrame(results)
    # sort by improvement pct descending
    df = df.sort_values("improvement_pct", ascending=False).reset_index(drop=True)

    out_png = os.path.join(args.out_dir, "clean_build_comparison.png")
    out_pdf = os.path.join(args.out_dir, "clean_build_comparison.pdf")
    make_plot(df, out_png, out_pdf)

    # write summary markdown
    os.makedirs(args.out_dir, exist_ok=True)
    md = []
    md.append(
        "| Project | Before (median ±95%CI ms) | After (median ±95%CI ms) | Improvement (ms) | Improvement (%) | runs (B/A) |"
    )
    md.append("|---|---:|---:|---:|---:|---:|")
    for _, r in df.iterrows():
        md.append(
            f"| {r['project']} | {r['before_median_ms']:.0f} [{r['before_ci_lo_ms']:.0f}, {r['before_ci_hi_ms']:.0f}] | {r['after_median_ms']:.0f} [{r['after_ci_lo_ms']:.0f}, {r['after_ci_hi_ms']:.0f}] | {r['improvement_ms']:.0f} | {r['improvement_pct']:.1f}% | {int(r['runs_before'])}/{int(r['runs_after'])} |"
        )

    summary_path = os.path.join(args.out_dir, "clean_build_summary.md")
    with open(summary_path, "w", encoding="utf-8") as fh:
        fh.write("\n".join(md))

    print("Wrote:", out_png, out_pdf, summary_path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
