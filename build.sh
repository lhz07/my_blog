#!/bin/bash
set -euo pipefail
cd blog/tailwind && npm run build-css-prod && cd -
cargo build --release --target x86_64-unknown-linux-musl --no-default-features --bin blog --bin search_utils
cargo run --release --bin search_utils
if [ -d "build" ]; then
    rm -r build
fi
mkdir -p build/blog
mkdir build/search_utils
cp -r blog/static build/blog/
cp -r blog/other_data build/blog/
cp target/x86_64-unknown-linux-musl/release/blog build/blog/
cp target/x86_64-unknown-linux-musl/release/search_utils build/blog/
cp -r blog/templates build/blog/
cp -r blog/posts build/blog/
cp -r search_utils/search build/search_utils
find "./build" -type f -name ".DS_Store" | while read -r shit_file; do
    echo There is a shit, delete it: $shit_file
    rm $shit_file
done
# we need this to avoid extended file attributes
tar --no-xattrs --no-mac-metadata -cf build.tar ./build && zstd build.tar && rm build.tar && mv build.tar.zst ./build/
echo "Build completed. The build artifacts are in the 'build' directory."
