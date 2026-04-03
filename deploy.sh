#!/bin/bash
set -Eeuo pipefail

function recover() {
    if [ -d "my_blog/blog" ]; then
        mv my_blog/blog my_blog/blog_old
    fi
    if [ -d "my_blog/search_utils" ]; then
        mv my_blog/search_utils my_blog/search_utils_old
    fi
}
trap recover ERR

mkdir -p my_blog
unzstd build.tar.zst && tar -xf build.tar
if [ -d "my_blog/blog" ]; then
    mv my_blog/blog_old my_blog/blog
fi
if [ -d "my_blog/search_utils" ]; then
    mv my_blog/search_utils_old my_blog/search_utils
fi
mv build/* my_blog/
chmod +x my_blog/blog/blog
systemctl --user restart my-blog.service
sleep 1
systemctl --user status my-blog.service
if [ -d "my_blog/blog_old" ]; then
    rm -r my_blog/blog_old
fi
if [ -d "my_blog/search_utils_old" ]; then
    rm -r my_blog/search_utils_old
fi
rm build.tar
rmdir build
