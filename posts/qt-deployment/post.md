一直以来，Qt(pronounced as "cute", not "cu-tee") 程序的打包部署都相当困难，今天我们进行一些简单的尝试。
由于静态编译较复杂，且可能涉及违反 Qt 的 LGPL 协议问题，故仅介绍动态编译。
Windows 和 macOS下都有官方提供的 windowsdeployqt 或 macdeployqt，一键部署还是比较方便的，但是 Linux 下官方没有提供类似的软件，可以尝试 [linuxdeployqt](https://github.com/probonopd/linuxdeployqt)，但是有两点限制：
1. 不支持部署 Wayland 程序
2. 必须使用当前仍受支持的最低 Ubuntu LTS进行部署

所以我们还是来看看 Qt 官方的解决方案吧。
#### 链接动态库
根据 Qt 的官方文档[Qt for Linux/X11 - Deployment](https://doc.qt.io/qt-6/linux-deployment.html)，动态编译的主要困难在于如何让编译出来的可执行文件能够找到需要的动态链接库。在文档中列举了三种方法：
1. Install the Qt libraries in one of the system library paths (e.g. `/usr/lib` on most systems).
2. Pass a predetermined path to the `-rpath` command-line option when linking the application. This will tell the dynamic linker to look in this directory when starting your application.
3. You can write a startup script for your application, where you modify the dynamic linker configuration (e.g., adding your application's directory to the `LD_LIBRARY_PATH` environment variable).

第一种方法其实是最不容易出现找不到库的错误的，只要使用者将所需的 Qt 及其他的动态库都安装到系统的 `/usr/lib` 下就可以。但是缺点也很明显，系统的动态库目录下可能已经有 Qt 库了（比如已经安装了 KDE ），显然不能重复安装，这个时候你的 Qt 程序是否能正确运行，就要看你使用的 Qt 版本和系统已有的 Qt 版本的兼容性如何了。

第三种方法也很简单，但是有以下两个缺点：
- **Note:** If your application will be running with "Set user ID on execution," and if it will be owned by root, then LD_LIBRARY_PATH will be ignored on some platforms. In this case, use of the LD_LIBRARY_PATH approach is not an option). 就是说这个方法有可能会失效，并且也没有任何补救的办法
- 不能直接运行可执行文件，而是要运行脚本来启动程序，显得非常不优雅，并且不符合使用直觉

下面我们来介绍第二种方法，Qt 的文档给出的方法是修改编译程序的命令，但是在较新版本的 Linux 下，CMake 编译时设置的 rpath 其实是 runpath，而 runpath 的优先级要低于环境变量 LD_LIBRARY_PATH和系统默认路径，这就意味着，如果你的程序使用的库较新，而实际运行的系统库较旧，且系统库里恰好有同名的运行库，就会导致严重的兼容性问题，因此我们使用 patchelf 来设置动态库的路径。

我们需要用到如下软件：
- `strings`
- `patchelf`
- `ldd`

对于不同的 GNU/Linux 发行版，它们可能包含在不同的软件包中。

首先使用`ldd ./application` 命令查找程序依赖，其中的`application`参数可以是可执行文件，也可以是动态库，对于 wayland 程序，我们主要是查找程序本身的依赖，以及`libqwayland-generic.so`(这是什么？请看插件一节) 的依赖。并将所需依赖复制到`lib`文件夹中去（假设我们的目录结构分为`bin`、`lib`、`plugins`三个文件夹）。
这里可以使用一个脚本来帮我们快速地复制这些动态库。
```bash
#!/bin/bash
exe="application"
des="$(pwd)"
deplist=$(ldd $exe | awk  '{if (match($3,"/")){ printf("%s "),$3 } }')
mkdir ./lib
cp $deplist $des/lib
```
将这个脚本和需要查找依赖的文件放在同一目录，并将`exe`的值改为该文件的名字，运行脚本即可。

**注意：是将脚本放在需要查找依赖的文件的目录下，如果将 Qt 的动态库文件移动到了别的目录下，可能导致查找到的依赖是系统的 Qt 库，而 Qt Creator 编译出来的可执行文件一般会链接到你用于开发的 Qt 库，一旦这两个库的版本不一致，就会导致不同版本的 Qt 库混用问题，程序根本无法运行。**

下面使用`patchelf`修改`rpath`，运行如下命令
```bash
patchelf --force-rpath --set-rpath '$ORIGIN/to/lib' ./application
```
`--force-rpath`的作用是确保设置的是rpath而不是runpath

其中的`$ORIGIN/to/lib`是程序查找动态库的路径，`$ORIGIN`是应用程序所在的目录

**注意：**
1. **`patchelf`会修改可执行文件，操作前建议备份原始文件**
2. **一定要使用单引号，否则终端会对`$ORIGIN`进行解释，导致路径设置出错**

完成后使用如下命令查看`rpath`是否成功修改
```bash
readelf -d ./application
```

如果你使用的 glibc 版本较高，应用程序在其他系统上运行时可能会出现兼容问题，因此还要设置`interpreter`
```bash
patchelf --set-interpreter /path/to/lib/ld-linux ./application
```
`lib`目录下有一个文件的名字是以`ld-linux`开头的，将它的路径填入即可。

**注意：此处的路径只能使用绝对路径，因此，如果希望程序能在较低版本的系统上运行，要么是能保证安装在固定位置，要么就使用低版本的系统进行编译，低版本的 glibc 对高版本有一定的兼容性。**

可通过如下命令查看是否设置成功
```bash
strings ./application | grep ld-linux
```

现在，如果你使用`ldd`命令查看程序依赖的话，应该会发现绝大多数的依赖都已经指向了你新设置的目录（当然要先把`lib`文件夹拷到对应的路径啊）。这意味着我们大部分的工作已经完成了，但是并不代表程序已经可以在任何系统上都正常运行了。

#### 插件
根据 Qt 官方文档，任何 Qt GUI程序运行时，都需要一个用于实现 QPA 层的插件。因此，我们需要将`path/to/qt/version/gcc_64/plugins`目录下的`platforms`文件夹复制到程序的运行目录下。其中就包含上面提到的`libqwayland-generic.so`。当然，你的 Qt 程序可能还用到了其他插件，需要一并拷贝到程序的运行目录下。
##### 如何知道需要哪些插件？
目前尚没有简单的解决办法，只能根据经验和插件名称进行推测，以及在不同系统上测试时有无功能缺失来了解。

#### 测试
测试系统需要是没有安装 Qt 环境的，并且系统版本最好旧一些。然后根据程序运行时的报错，来添加所需的动态库。

比如我尝试在 Ubuntu 22.04.5 上运行我的程序，出现如下错误：
```shell
QSystemTrayIcon::setVisible: No Icon set
Could not create decoration from factory! Running with no decorations.
```
第一个错误，推测可能是 iconengines 缺少依赖，使用`ldd`查找对应的依赖，发现少了`libQt6Svg.so.6`
第二个错误，推测可能是 wayland-decoration-client 缺少依赖，查找后发现也是少了`libQt6Svg.so.6`
将用于开发的 Qt 库目录下的`libQt6Svg.so.6`文件复制到`lib`文件夹中，程序不再报错。

#### 其他

##### 设置 rpath 后有部分动态库还是找不到
使用如下命令手动添加找不到的库
```bash
patchelf --add-needed /path/to/library.so ./application
```

##### 调整 plugins 文件夹的位置
比如我不想在程序运行目录下放 plugins 文件夹，那么我可以在程序目录下放一个`qt.conf`文件，在其中指定 plugins 文件夹的位置

```conf
[Paths]
Prefix = ./../                       # 下面的设置项的路径前缀
Plugins = plugins                  # plugins文件夹的路径
```
实际上的 plugins 文件夹路径等于 Prefix 加上 Plugins，即 `./../plugins`

等同于这么写

```conf
[Paths]
Prefix = ./                       # 下面的设置项的路径前缀
Plugins = ../plugins                  # plugins文件夹的路径
```
