import glob
import json
from pydantic import BaseModel


class Metrics(BaseModel):
    actions_executed: int
    cpu_time_in_ms: int
    wall_time_in_ms: int


def parse_metrics(data: dict) -> Metrics:
    return Metrics(
        actions_executed=data["actionSummary"]["actionsExecuted"],
        cpu_time_in_ms=data["timingMetrics"]["cpuTimeInMs"],
        wall_time_in_ms=data["timingMetrics"]["wallTimeInMs"],
    )


build_metrics = dict()
for path in sorted(glob.glob("data/experiment/*/stats/incre_build/*.json")):
    project_name = path.split("/")[2]

    is_after = "-after" in path

    with open(path, "r") as f:
        for line in f.readlines():
            if not line.strip():
                continue
            if "buildMetrics" not in line:
                continue
            item = json.loads(line)

            if "buildMetrics" in item:
                metrics = parse_metrics(item["buildMetrics"])

                suffix = "after" if is_after else "before"

                if project_name not in build_metrics:
                    build_metrics[project_name] = dict()
                if suffix not in build_metrics[project_name]:
                    build_metrics[project_name][suffix] = []
                
                build_metrics[project_name][suffix].append(metrics)

def compute_stats(metrics_list):
    total = sum(metrics_list)
    avg = total / len(metrics_list) if metrics_list else 0
    min_v = min(metrics_list) if metrics_list else 0
    max_v = max(metrics_list) if metrics_list else 0
    return total, avg, min_v, max_v

for project_name, metrics in build_metrics.items():
    before_metrics = metrics.get("before", [])
    after_metrics = metrics.get("after", [])

    n_before = len(before_metrics)
    n_after = len(after_metrics)

    before_actions = [m.actions_executed for m in before_metrics]
    after_actions = [m.actions_executed for m in after_metrics]
    before_cpu = [m.cpu_time_in_ms for m in before_metrics]
    after_cpu = [m.cpu_time_in_ms for m in after_metrics]
    before_wall = [m.wall_time_in_ms for m in before_metrics]
    after_wall = [m.wall_time_in_ms for m in after_metrics]

    def deltas(before, after):
        return [a - b for a, b in zip(after, before)] if before and after else []

    actions_delta = deltas(before_actions, after_actions)
    cpu_delta = deltas(before_cpu, after_cpu)
    wall_delta = deltas(before_wall, after_wall)

    total_before_actions, avg_before_actions, min_before_actions, max_before_actions = compute_stats(before_actions)
    total_after_actions, avg_after_actions, min_after_actions, max_after_actions = compute_stats(after_actions)
    total_before_cpu, avg_before_cpu, min_before_cpu, max_before_cpu = compute_stats(before_cpu)
    total_after_cpu, avg_after_cpu, min_after_cpu, max_after_cpu = compute_stats(after_cpu)
    total_before_wall, avg_before_wall, min_before_wall, max_before_wall = compute_stats(before_wall)
    total_after_wall, avg_after_wall, min_after_wall, max_after_wall = compute_stats(after_wall)

    def print_delta_stats(name, delta):
        if delta:
            non_zero = sum(1 for d in delta if d != 0)
            print(f"    {name} Delta: min={min(delta)}, max={max(delta)}, non-zero={non_zero}")
        else:
            print(f"    {name} Delta: N/A")

    print(f"Project: {project_name}")
    print(f"  Count: Before={n_before}, After={n_after}")
    print(f"  Actions Executed: Before total={total_before_actions}, avg={avg_before_actions:.2f}; After total={total_after_actions}, avg={avg_after_actions:.2f}")
    print_delta_stats("Actions Executed", actions_delta)
    print(f"  CPU Time (ms): Before total={total_before_cpu}, avg={avg_before_cpu:.2f}; After total={total_after_cpu}, avg={avg_after_cpu:.2f}")
    print_delta_stats("CPU Time (ms)", cpu_delta)
    print(f"  Wall Time (ms): Before total={total_before_wall}, avg={avg_before_wall:.2f}; After total={total_after_wall}, avg={avg_after_wall:.2f}")
    print_delta_stats("Wall Time (ms)", wall_delta)
    print()