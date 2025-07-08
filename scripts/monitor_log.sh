nlines=${2:-30000}
tail -n $nlines $1 | grep -e "Processing node" -e "Remaining candidates" -e "Committed changes"