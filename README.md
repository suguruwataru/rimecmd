![Recording](vhs.gif)

rimecmd是基于[librime](https://github.com/rime/librime)的Linux命令行程序。

rimecmd有一个可以在终端界面上使用的界面，不过单单是这个界面的话不能说使用体验很好。
但是，在使用其终端界面的同时，也可以通过JSON与其通信，来与librime交流。

目前，rimecmd仅暴露了librime的部分API. 毕竟我写它主要是为了让
[nvim-rimecmd](https://github.com/suguruwataru/nvim-rimecmd)
使用。只要暴露的API能把这个插件做出来，别的部分我也不太着急。

# 构建和安装

rimecmd依赖于librime, 所以构建和运行都需要你的系统中安装有librime。

通过
```
cargo build --release
```
来构建可执行程序。`target/release/rimecmd`就是构建出来的可执行程序。可以把它复制
进你的环境变量`PATH`中的某个目录里。
