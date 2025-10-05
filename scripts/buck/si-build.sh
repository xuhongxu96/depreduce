set -e

buck2 uquery 'kind("rust_(binary|library|test)", set("//bin/..." "//lib/..."))' | xargs buck2 build