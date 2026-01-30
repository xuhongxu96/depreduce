import os
import json
import sys


def compress_json_file(input_path, output_path):
    """Compresses a single JSONL file by removing whitespace and the 'target' key."""
    with open(input_path, "r") as f_in, open(output_path, "w") as f_out:
        for line in f_in:
            if "buildMetrics" not in line:
                continue
            f_out.write(line)
    print(f"Compressed {input_path} to {output_path}")


def copy_file(input_path, output_path):
    """Copies a file from input_path to output_path."""
    try:
        with open(input_path, "r") as f_in, open(output_path, "w") as f_out:
            content = f_in.read()
            index = content.find("\nbuild_metrics {")
            if index != -1:
                content = content[index:]
            f_out.write(content)

        print(f"Compressed {input_path} to {output_path}")
    except Exception as e:
        print(
            f"An unexpected error occurred while copying {input_path}: {e}",
            file=sys.stderr,
        )


def main():
    """
    Compresses JSON files in the specified directory by removing whitespace
    and copies other file types to a new directory.
    """
    input_dir_incre_build = "data/experiment/angular/stats/incre_build/"
    output_dir_incre_build = "data/experiment/angular/stats/incre_build_opt/"

    selected_input_dir = None
    selected_output_dir = None

    if os.path.exists(input_dir_incre_build):
        selected_input_dir = input_dir_incre_build
        selected_output_dir = output_dir_incre_build
    else:
        print(f"Input directory does not exist: {input_dir_incre_build}", file=sys.stderr)
        sys.exit(1)

    if not os.path.exists(selected_output_dir):
        os.makedirs(selected_output_dir)
        print(f"Created output directory: {selected_output_dir}")

    for filename in os.listdir(selected_input_dir):
        input_path = os.path.join(selected_input_dir, filename)
        output_path = os.path.join(selected_output_dir, filename)

        if not os.path.isfile(input_path):
            continue

        if filename.endswith(".json"):
            compress_json_file(input_path, output_path)
        elif filename.endswith(".txt"):
            copy_file(input_path, output_path)


if __name__ == "__main__":
    main()
