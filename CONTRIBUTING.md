在`git commit`之前，应当先`git config core.hooksPath githooks`。`githooks`目录
里有一个用于`commit`之前的检查的脚本，可以保证`commit`进来的代码满足一定的要求。

rimecmd基本上是用Rust语言写的，可以使用标准的
```
cargo build
```
来构建可执行程序。

你也可以直接`cargo test`。这样的话，可执行程序被构建出来之外，还会直接运行测试。

`cargo test` 只会执行那些不直接与librime交互的测试。librime本身并不是特别支持从
多个线程调用其API。`cargo`默认会多线程同时运行测试，这个时候调用librime API的话
就会出问题。因此，那些测试被标记了`#[ignored]`。想要运行那些测试的话，

```
cargo test -- --ignored --test-threads=1
```
