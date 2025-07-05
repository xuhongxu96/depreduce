# Experiment Design

We conducted an empirical study of depreduce on various repositories that use Bazel as their build system..

## Repository Selection

Key selection criteria included Bazel usage, repository size, activity level, and popularity.

We first identified repositories containing BUILD, *.bazel, or *.bzl files by executing SQL queries on GitHub Activity data via Google BigQuery, 
resulting in a total of 4,450 repositories.

```sql
SELECT DISTINCT repo_name 
    FROM `bigquery-public-data.github_repos.files` 
    WHERE (ENDS_WITH(ref, "main") 
            or ENDS_WITH(ref, "master")) 
        and (ENDS_WITH(path, "/BUILD") 
            or ENDS_WITH(path, ".bazel") 
            or ENDS_WITH(path, ".bzl")) 
    LIMIT 10000
```

Next, we used the GitHub API to retrieve metadata for these repositories and excluded any that were duplicates (due to renaming), forked, archived, disabled, or deleted. 
After filtering, 1,592 repositories remained.

To focus our evaluation on more actively maintained and relatively large projects, we further filtered the repositories to those with a latest push date later than January 1st, 2025, and a size greater than 1,000 KB. This final selection yielded __ repositories.