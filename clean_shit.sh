if [ -z "$1" ]; then
    echo "Usage: $0 /path/to/folder"
    exit 1
fi

INPUT_DIR="$1"

find "$INPUT_DIR" -type f -name ".DS_Store" | while read -r shit_file; do
    echo There is a shit, delete it: $shit_file
    rm $shit_file
done
