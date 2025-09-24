#! /bin/bash
cd tailwind && npm run build-css-prod && cd -
cargo build --release --target x86_64-unknown-linux-musl
if [ -d "build" ]; then
    rm -r build
fi
mkdir build
cp -r static build/static
cp target/x86_64-unknown-linux-musl/release/my_blog build/my_blog
cp -r templates build/templates
cp -r posts build/posts
echo "Build completed. The build artifacts are in the 'build' directory."
