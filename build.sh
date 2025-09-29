#!/bin/bash
cd tailwind && npm run build-css-prod && cd -
cargo build --release --target x86_64-unknown-linux-musl
if [ -d "build" ]; then
    rm -r build
fi
mkdir build
cp -r static build/static
cp -r other_data build/other_data
cp target/x86_64-unknown-linux-musl/release/my_blog build/my_blog
cp -r templates build/templates
cp -r posts build/posts
find "./build" -type f -name ".DS_Store" | while read -r shit_file; do
    echo There is a shit, delete it: $shit_file
    rm $shit_file
done
echo "Build completed. The build artifacts are in the 'build' directory."
