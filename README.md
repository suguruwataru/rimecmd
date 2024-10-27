![Recording](vhs.gif)

rimecmd是基于[librime](https://github.com/rime/librime)的Linux命令行程序。

rimecmd有一个可以在终端界面上使用的界面，不过单单是这个界面的话不能说使用体验很好。但是，在使用其终端界面的同时，也可以通过JSON与其通信，来与librime交流。

目前，rimecmd仅暴露了librime的部分API. 毕竟我写它主要是为了让[nvim-rimecmd](https://github.com/suguruwataru/nvim-rimecmd)使用。只要暴露的API足够把这个插件做出来，别的部分我也不太着急。

# 构建和安装

rimecmd依赖于librime, 所以构建和运行都需要你的系统中安装有librime。

通过
```
cargo build --release
```
来构建可执行程序。`target/release/rimecmd`就是构建出来的可执行程序。可以把它复制进你的环境变量`PATH`中的某个目录里。

# 架构

rimecmd有使用一个客户-服务端架构。客户端进程与服务端进程通过Unix domain socket通信。具体的Unix domain socket路径可以通过`--print-config`命令行参数查看。在这个路径下没有文件的情况下，使用rimecmd会启动一个服务端进程。

之所以使用这样的架构是因为，librime的运行需要与一个名叫“user data directory”的目录下的文件系统交互。如果有多个进程同时与同样的文件交互的话，显然容易出现数据遭到破坏的问题。因此，rimecmd希望尽可能保证只有服务端一个进程会接触“user data directory”下的文件。

目前，rimecmd还不支持更改“user data directory”的路径。现在正在使用的路径可以通过`--print-config`查看。
