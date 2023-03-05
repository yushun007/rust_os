# 内核测试

本章讲述在 no_std 环境下进行单元测试和集成测试的方法.通过将 Rust 的自定义测试框架来在内核中执行一些测试函数.

## Rust 中的测试

Rust 有一个内置的**测试框架(biuld-in test framework)**:无需任何设置就可以进行单元测试,只需要创建一个通过 assert 来检查结果的函数并在函数头部加上`#[test]`属性即可.然后`cargo test`会自动找到并执行 crate 中的所有测试函数

但是对于`no_std`应用,有点复杂.现在问题是,rust 的测试框架回音是的调用内置的`test`库,但是这个库依赖于标准库.也就是说我们的`#[no_std]`内核无法使用默认的测试框架.

## 自定义测试框架

luckily,Rust 支持通过使用不稳定的**自定义测试框架(custom_test_frameworks)**功能来替换默认的测试框架.该功能不需要额外的库,因此在`#[no_std]`环境中他也可以工作.他的工作原理是手机所有标注了`#[test_case]`属性的函数,然后将这个测试函数的列表作为参数传递给用户指定的 runner 函数.因此它实现了对测试过程的最大控制.

与默认的测试框架相比,他的缺点是有一些高级功能例如`should_panic,tests`都不可用了.相对的,如果需要这些功能,我们需要自己实现.

要为我们的内核实现自定义测试框架,我们需要将如下代码添加到我们的`main.rs`中:

```rust
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]

#[cfg(test)]
fn test_runner(tests: &[&dyn fn()]){
    println!("Running {} tests",tests.len());
    for test in tests{
        test();
    }
}
```

我们的 runner 会打印一些简短的 debug 信息然后调用列表中的测试函数.参数类型`&[&dyn Fn()]`是`Fn()`trait 的一个 trait object 引用的一个 slice.它基本上可以被看作是一个可以像函数一样被调用的类型的引用列表.由于这个函数在不进行测试的时候没什么用,这里我们使用`#[cfg(test)]`属性保证他只会出现在测试中.

此时我们运行 cargo test,已经可以运行成功.但是没有任何 test 信息.这是因为我们的入口仍然是`_start`函数--自定义框架会生成一个`main`函数来调用`test_runner`,但是我们的入口是`_start`所以`main`就被忽略了.

为了修复这个问题,我们需要通过`reexport_test_harness_main`属性来将生成的函数的名称更改为与`main`不同的名称.然后我们可以在`_start`函数中调用这个重命名的函数:

```rust
#![reexport_test_harness_main = "test_main"]
#[no_mangle]
pub extern "C" fn _start() -> !{
    test_main();
    println!("hello world1{}","!");
    panic!("Some panic message!");
}
```

这里我们将测试框架的入口函数名字设置为`test_main`,并在我们的`_start`入口点调用它.通过使用**条件编译(conditional compilation)**,我们能够只在测试环境中调用它.

创建第一个测试函数:

```rust
```
