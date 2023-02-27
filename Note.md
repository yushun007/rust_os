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

## 添加 start 语言项

当运行一个程序时,第一个被调用的函数是 main 函数,其实大多是语言都有运行时系统(runtime system),例如 java 的 GC 系统或者 Go 的协程(goroutine),会在main 函数之前运行,已初始化程序.

在一个典型的使用 rust 标准库的程序中,程序运行是从一个名为`crt0`的运行时库开始的.`crt0`表示`C runtime zero`,他建立一个适用于运行 C 语言程序的环境,包括栈的创建和可执行程序的参数的传入.在这之后,这个运行时库会调用 rust 的运行时入口函数,这个入口函数被称为 start 语言项("start" language item).rust 运行时很小,只有较少的功能,例如爆栈检测和打印栈轨迹(stack trace),这之后会调用main 函数.

裸机程序并不能访问 rust 运行时或者 crt0 库,所以需要自定义入口函数.只实现一个"start"语言向并不能满足运行需求,rust 仍然需要 crt0 库.所以我们需要重写 crt0 库和它定义的入口函数.

## 重写入口函数

由于我们不再使用 main 作为入口函数,所以需要使用#![no_main]告诉编译器

```rust
#![no_std]
#![no_main]

use core::panic::PanicInfo;
#[panic_handler]
fn panic(_info:&PanicInfo)->!{
  loop{}
}

```

为了重写操作系统的入口函数,我们编写一个`_start`函数:

```rust
#[no_mangle]
pub extern "C" fn _start()->!{
  loop{}
}
```

如果使用 clang 编译器这里需要换成`_main`函数.

同时我们使用`#[no_mangle]`标记此函数,来对他禁用名称重整(name mangle)--确保 rust 编译器输出一个名为`_start`的函数;否则编译器可能会生成`_ZN3blog_os4_start7hb173fedf945531caE`类似的函数名,这会使得链接器无法正确的识别.

另外`exter "C"`表示这个函数使用 C语言调用约定.而不是 rust 语言的调用约定.
此时如果编译,会报出一堆连接错误:

```shell
error: linking with `cc` failed: exit status: 1
  |
  = note: "cc" "-arch" "x86_64" "-m64" "/var/folders/zz/pt9z_x797lg88_h57819sqtw0000gn/T/rustcHiZvcQ/symbols.o" "/Users/yushun/develop/rust_os/target/debug/deps/rust_os-f84e97690f6b4197.s45c1hgy4f2j2fe.rcgu.o" "-L" "/Users/yushun/develop/rust_os/target/debug/deps" "-L" "/Users/yushun/.rustup/toolchains/stable-x86_64-apple-darwin/lib/rustlib/x86_64-apple-darwin/lib" "/Users/yushun/.rustup/toolchains/stable-x86_64-apple-darwin/lib/rustlib/x86_64-apple-darwin/lib/librustc_std_workspace_core-6dd5c0cef5b5f8b3.rlib" "/Users/yushun/.rustup/toolchains/stable-x86_64-apple-darwin/lib/rustlib/x86_64-apple-darwin/lib/libcore-7d2b712101daf86c.rlib" "/Users/yushun/.rustup/toolchains/stable-x86_64-apple-darwin/lib/rustlib/x86_64-apple-darwin/lib/libcompiler_builtins-9b0bf4523e9805e9.rlib" "-L" "/Users/yushun/.rustup/toolchains/stable-x86_64-apple-darwin/lib/rustlib/x86_64-apple-darwin/lib" "-o" "/Users/yushun/develop/rust_os/target/debug/deps/rust_os-f84e97690f6b4197" "-Wl,-dead_strip" "-nodefaultlibs"
  = note: ld: entry point (_main) undefined. for architecture x86_64
          clang: error: linker command failed with exit code 1 (use -v to see invocation)
          

error: could not compile `rust_os` due to previous error
```

### 连接器错误

连接器是一个程序,它将汇编器生成的目标文件组成一个可执行文件.不同的操作系统如 windows,linux,macOS,规定了不同的可执行文件的格式,因此也有不同的连接器,抛出不同的错误,但是根本原因是相同的:**连接器默认配置嘉定程序以来于 C 语言的运行时环境,但是我们的程序并不依赖它**

为了解决链接错误,我们需要告诉连接器,我们现在不需要 C语言运行时环境.

### 编译为裸机目标

默认情况下,rust 尝试适配当前系统的环境,编译可执行程序.例如在 x86_64平台 windows 系统,rust 会编译一个.exe 文件,并使用 x86_64 指令集.这个环境又被称作宿主系统("host" system)

为了描述不同的环境,rust 使用一个成为目标三元组的字符串.要查看当前系统可以执行`rustc --version --verbose`.

此时如果我们执行 cargo build rust 会尝试为当前系统的三元组编译,并假设底层有一个类似于 windows 或者 linux 的系统提供 C语言运行环境.此时我们就需要指定一个没有操作系统的环境.

这样的运行环境成为裸机环境,例如目标三元组 `thubv7em-none-eabihf`一个 ARM 嵌入式系统.这里的 none 即表示没有操作系统.要使用它我们需要使用 rustup 添加它:

```shell
rustup target add thumbv7em-none-eabihf
```

### 使用连接器命令解决链接错误

- linux:
  - `cargo rustc -- -C link-arg=-nostartfiles`
  - 告诉连接器不要使用 C 启动例程功能
- windows:
  - `cargo rustc -- -C link-args="/ENTRY:_start /SUBSYSTEM:console"`
  - 选定 console 子系统,设定入口
- macOS:
  - `cargo rustc -- -C link-args="-e __start -static -nostartfiles"`
  - 指定入口函数,指定静态链接,指定不使用 C 启动例程功能

### 统一编译命令

由于各个平台的编译命令不尽相同,手动输入难免出错,所以使用,配置文件来解决此问题,创建一个`.cargo/config.toml`文件来指定不同平台的编译参数:

```rust
[target.'cfg(target_os = "linux")']
rustflags = ["-C","link-arg=-nostartfiles"]
[target.'cfg(target_os = "windows")']
rustflags = ["-C","link-args=/ENTRY:_start /SUBSYSTEM:console"]
[target.'cfg(target_os = "macos")']
rustflags = ["-C","link-args=-e __start -static -nostartfiles"]
```

