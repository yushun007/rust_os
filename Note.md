# 裸机程序

- 添加#![no_std]属性之后,编译会提示 println!宏为标准库提供,此时无法使用,因此删除此宏调用

- 删除 println!宏之后,cargo biuld 编译会报:

  ```shell
    error: `#[panic_handler]` function required, but not found

    error: language item required, but not found: `eh_personality`
      |
      = note: this can occur when a binary crate with `#![no_std]` is compiled for a target where `eh_personality` is defined in the standard library
      = help: you may be able to compile for a target that doesn't need `eh_personality`, specify a target with `--target` or in `.cargo/config`

    error: could not compile `rust_os` due to 2 previous errors
  ```

  - 此错误表示我们缺少一个#[panic_handler]函数和一个语言选项`eh_personality`

## 实现 panic 函数

- panic_handler 属性定义了一个函数,他会在一个 panic 发生时被调用.标准库中提供了自己的 panic 处理函数,但在 no_std 中需要自定义.

```rust
    use core::panic::PanicInfo;
    #[panic_handler]
    fn panic(_info:&PanicInfo)->!{
        loop{}
    }
```

- PanicInfo 中包含了 panic 发生时的文件名,代码行数可选地错误信息.这个函数从不返回所以被定义为**发散函数**.如下所示发散函数的返回值被定义为`never`类型('never'type),记为!

## eh_personality语言项

语言项是一些编译器需求的特殊函数或者类型.例如,rust 的 Copy trait 是一个这样的语言项,告诉编译器哪些类型需要遵循**复制语义**,rust中 copy trait 的实现是由一个#[lang = "copy"]属性定义为一个语言项,达到与编译器联系的目的.

我们可以自行实现语言项,但是 rust 现在的语言项支持并不稳定,其不会经过编译期的类型检查.

我们碰到的`eh_personality`语言项被用于实现**栈展开**(所谓栈展开就是释放栈上的变量).标准库中,当 panic 发生时,rust 将使用栈展开,来运行栈上所有活动的变量的**析构函数**以确保所有内存被释放,允许调用程序的父进程捕获 panic,处理并继续运行.但是栈的展开是一个复杂的过程,如 linux 的 libunwind 或者 windows 的结构化异常处理(structure exception handling),通常需要操作系统库的支持.因此我们暂时不使用此功能.

TODO:libunwind

### 禁用栈展开

栈展开并不是迫切的需求.因此,rust 提供了在 panic 时终止(abort on panic)选项.打开方式:

```cargo
    [profile.dev]
    panic = "abort"

    [profile.release]
    panic = "abort"
```

这些选项将开发配置(dev profile)和 release 配置(relcease profile)的 panic 策略设为 abort.dev 配置适用于 cargo build,而 release 配置适用于 cargo build --release.