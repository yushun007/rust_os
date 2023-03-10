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
#[test_case]
fn trivial_assertion(){
    print!("trivial assertion ...");
    assert_eq!(1,1);
    println!("[OK]");
}
```

但是这里有个问题,`test_runner`会将结果返回给`test_main`函数,而这个函数又返回到`_start`入口点函数--这样我们就进入一个死循环,因为入口点函数是不允许返回的.这将导致:测试完毕后没有自动退出.

## 退出 QEMU

现在我们的`_start`函数最后是一个死循环,所以每次执行完`cargo test`之后要手动关闭 QEMU;但是我们还想再没有用户交互的脚本环境下执行`cargo test`.解决这个问题的最佳方式,是实现一个合适的方法来管比我们的操作系统--unlucky,这个方法实现起来相对有些复杂,因为这要求我们实现对 APM 或者 ACPI 电源管理标准的支持.

有一种绕开这些问题的方法:QEMU 支持一种名为`isa-debug-exit`的特殊设备,他提供了一种从**客户系统(guest system)**退出 QEMU 的简单方法.为了使用这个设备,我们需要向 QEMU 传递一个`-device`参数.当然也可以通过将`package.metadata.bootimage.test-args`配置关键字添加到`Cargo.toml`中:

```config
[package.metadata.bootimage]
test-args = ["-device","isa-debug-exit,iobase=0xf4,iosize=0x04"]
```

在传递设备名(isa-debug-exit)的同时,我们还传递了两个参数,iobase 和 iosize.这两个参数指定了一个 **I/O端口**,内核将通过他来访问设备.

## I/O 端口

在 x86 平台上,CPU 和外围硬件通信通常有两种方式,**内存映射I/O**和**端口映射I/O**,上一章我们已经使用过内存映射 I/O(VGA 文本缓冲区).

与内存映射不同,端口映射I/O使用独立的 I/O 总线来进行通信.每个外围设备都有一个或数个端口号.CPU 采用特殊的 in 和 out 指令来和端口通讯,这些指令要求一个端口号和一个字节的数据作为参数(有些这种指令的变体也许允许发送`u16`或者`u32`长度的数据)

isa-debug-exit设备使用的就是端口映射 I/O.其中,`iobase`参数指定了设备对应的端口地址,而`iosize`则制定了端口的大小.

## 使用退出(Exit)设备

isa-debug-exit设备功能非常简单.当一个 value 写入`iobase`指定的端口时,它会导致 QEMU 以**退出状态(exit status)**`(value<<1)|1`退出.也就是说,当我们向端口写入 0 时,QEMU 将以退出的状态`(1<<1)|1=3`退出.

这里使用 x86_64crate 提供的抽象,而不是手动调用`in,out`指令.为了添加该 crate 的依赖,我们可以将其添加到我们`Cargo.toml`中:

```config
[dependencies]
x86_64 = "0.14.2"
```

现在我们就可以使用 crate 中提供的 `port`类型创建一个`exit_qemu`函数:

```rust
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode){
    use x86_64::instructions::port::Port;
    unsafe{
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}
```

该函数在 0xf4 处创建了一个新的端口,该端口同时也是`isa-debug-exit`设备的`iobase`.然后它会向端口写入传递的退出代码.这里我们使用 u32 类型传递,因为我们之前已经将`isa-debug-exit`设备的`iosize`指定为了 4 字节.因为端口的读写都是编译器无法判断安全与否的,所以需要 unsafe 语句块.

为了指定退出状态,我们创建了一个`QemuExitCode`枚举类.思路大体上是,如果所有的测试均成功,就以成功退出吗退出;否则以失败码退出.

现在修改我们的测试函数:

```rust
#[cfg(test)]
fn test_runner(tests: &[&dyn Fn()]){
    println!("Running {} tests",tests.len());
    for test in tests{
        test();
    }
    exit_qemu(QemuExitCode::Success);
}
```

## 成功退出(Exit)码

此时我们运行`cargo test`cargo 会告诉我们测试失败,这是因为`cargo test`会将所有非 0 返回码视为测试失败.
解决办法是:`bootimage`提供了一个`test-success-exit-code`可以将指定的退出代码映射为 0:

```config
[package.metadata.bootimage]
test-success-exit-code = 33 #(0x10<<1)|1
```

## 打印到控制台

要在控制台上查看测试输出,我们需要某种方式将数据从内核发送到宿主机.有多种方法可以实现这一点,例如通过 TCP 网络接口来发送数据.但是设置网络对战是一项很复杂的任务,这里我们可以选择更简单的解决方案.

### 串口

发送数据一个简单的方式是通过**串行端口**,这是一个现代 PC 很少存在的标准接口,嵌入式还存在很多.串口非常易于编程,QEMU 可以将通过串口发送的数据重定向到宿主机的标准输出或者文件中.

用来实现串行接口的芯片成为`UARTs`.x86 上,有很多`UART`模型,但是这些模型不同之处都是我们用不到的高级特性,目前通用的`UART`都会兼容`16550 UART`,所以我们使用这个模型.

我们使用`uart_16550`crate 来初始化`UART`,并通过串口来发送数据.添加`uart_16550`crate:

```config
uart_16550 = "0.2.0"
```

这个包包含了一个代表UART寄存器的`SerialPort`结构体,但是我们仍然需要自己来创建一个相应的对象.

```rust
//in src/main.rs
mod serial;
//in src/serial.rs
use uart_16550::SerialPort;
use spin::Mutex;
use lazy_static::lazy_static;

lazy_static!{
    pub static ref SERIAL1:Mutex<SerialPort> = {
        let mut serial_port = unsafe{ SerialPort::new(0x3F8)};
        serial_port.init();
        Mutex::new(serial_port)
    };
}

```

就像 VGA 文本缓冲区一样,我们使用`lazy_static`和一个自旋锁来创建一个`static SERIAL1`实例.通过使用`lazy_static`我们保证`init`方法只会在该对象第一次被使用时被调用.

和`isa-debug-exit`设备一样,UART 也是通过端口 I/O进行编程.由于 UART 相对来讲更加复杂,它使用多个 I/O 端口来对不同的设备寄存器进行编程.`unsafe`的`SerialPort::new`函数需要 UART 的第一个 I/O端口的地址作为参数,从改地址中可以计算出所有所需端口的地址.我们传递端口地址为`0x3F8`,改地址是第一个串行接口的标准端口号.

为了使串口更加易用,我们添加两个宏:`serial_print!,serial_println!`:

```rust
#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments){
    use core::fmt::Write;
    SERIAL1.lock().write_fmt(args).expect("Printing to serial failed!");
}


#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::_print(format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! serial_println {
    () => {
        $crate::serial_print!("\n")
    };
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt,"\n")));
    ($fmt:expr,$($arg:tt)*) => ($crate::serial_print!(
        concat!($fmt,"\n"),$($arg)*
    ));
}
```

和之前的`print!,println!`类似.由于`SerialPort`类型已经实现了`fmt::Write`trait,所以我么你不需要提供自己的实现.

现在我们可以从测试代码想串行接口打印而不是向 VGA 文本缓冲区打印了:

```rust
#[cfg(test)]
fn test_runner(tests: &[&dyn Fn()]){
    serial_println!("Running {} tests",tests.len());
    for test in tests{
        test();
    }
    exit_qemu(QemuExitCode::Success);
}

#[test_case]
fn trivial_assertion(){
    serial_print!("trivial assertion ...");
    assert_eq!(1,1);
    serial_println!("[OK]");
}
```

### QEMU参数

为了查看 QEMU 的串行输出,我们需要使用`-serial`参数将输出重定向到 stdout:

```config
[package.metadata.bootimage]
test-args = ["-device","isa-debug-exit,iobase=0xf4,iosize=0x04","-serial","stdio"]
```

此时我们使用`cargo test`可以看到终端输出了测试结果,但是测试失败的结果仍然会打印在 QEMU 中.这是因为我们的panic handler 函数还是使用的`println!`宏.

### 在 panic时打印一个错误信息

为了在 panic 时使用错误信息来推出 QEMU,我们使用**条件编译**在测试模式和非测试模式中使用不同的 panic 处理方式:

```rust
//panic处理函数
#[cfg(not(test))]
#[panic_handler]
fn panic(_info:&PanicInfo)->!{
    println!("{}",_info);
    loop {
    }
}
#[cfg(test)]
#[panic_handler]
fn panic(_info:&PanicInfo)->!{
    serial_println!("[failed]\n");
    serial_println!("Error: {}",_info);
    loop {
    }
}
```

## 隐藏 QEMU

由于使用`isa-debug-exit`设备和串行设备来报告完整的测试结果,所以我们不再需要 QEMU 的窗口了.通过向其传递`-display none`参数隐藏窗口:

```config
test-args = ["-device","isa-debug-exit,iobase=0xf4,iosize=0x04","-serial","stdio","-display","none"]
```

## 超时

由于`cargo test`会等待`test runner`退出,如果一个测试永远不返回那么它将阻塞`test runner`,幸运的是,在实际应用中这并不是大问题,因为无限循环通常很容易避免.无限循环会发生在一下几种不同的情况中:

- bootloader加载内核失败,导致系统不停地重启;
- BIOS/UEFI 固件加载 bootloader 失败,同样会无限重启;
- CPU 在某些函数中进入 loop{}中.
- 硬件触发了系统重置,例如未捕获 CPU 异常时.

bootimage 默认会有 5 分钟的超时检测,如果超时会向控制台输出`Timed Out`错误.
这个时间可以通过配置设置:

```config
test-timeout=120 $(in seconds)
```

## 自动添加打印语句

`trivial_assertion`测试仅能使用`serial_print!,serial_println!`输出自己的状态信息:

为每个测试手动添加固定的日志太麻烦,我们修改一下`test_runner`把这部分逻辑改进一下,使其可以自动添加日志输出.那么我们先建立一个`Testable trait`:

```rust
pub trait Testable {
    fn run(&self)->();
}

impl<T> Testable for T where T:Fn(),{
    fn run(&self){
        serial_print!("{}..\t",core::any::type_name::<T>());
        self();
        serial_println!("[OK]");
    }
}
```

我们定义一个trait,`Testable`其中包含一个函数`run`,实现中指定只有实现了`Fn()`trait 的泛型可以使用这个实现
我们实现的`run`函数中,首先使用`any::type_name`输出函数名,这个函数事实上是被编译器实现的,可以返回任意类型的字符串形式.对于函数而言,其类型的字符串形式就是它的函数名,而函数名也正是我们想要的测试用例名称.至于`\t`代表制表符,为后面的`[OK]`添加点距离.

输出函数名之后,我们通过`self()`调用了测试函数本身,该调用方式属于`Fn()`trait 独有,如果测试函数顺利执行,则输出`[OK]`.

最后一步为`test_runner`的参数附加上`Testable trait`:

```rust
#[cfg(test)]
fn test_runner(tests: &[&dyn Testable]){
    serial_println!("Running {} tests",tests.len());
    for test in tests{
        test.run();
    }
    exit_qemu(QemuExitCode::Success);
}
```

由于我们已经完成了收尾的输出自动化,所以`trivial_assertion`就可以删掉这些输出.

```rust
#[test_case]
fn trivial_assertion(){
    assert_eq!(1,1);
}
```

## 测试 VGA 缓冲区

现在我们已经有了一个可以工作的测试框架了,我们可以为我们的 VGA 缓冲区实现创建一些测试.首先创建一个非常简单的测试来验证`println!`是否正常运行而不会 panic:

```rust
//in src/vga_buffer.rs

#[test_case]
fn test_print_ln_simple(){
    println!("test_println_simple output");
}
```

为了确保打印很多行也不会出问题,所以我们多输出一些:

```rust
//in src/vga_buffer.rs

#[test_case]
fn test_println_simple(){
    for _ in 0..200{
        println!("test_println_simple output");
    }
}
```

还可以创建另一个测试函数,来验证打印的几行字符是否真的出现在了屏幕上:

```rust
#[test_case]
fn test_println_output(){
    let S = "Some test string that fits on a single line";
    println!("{}",S);
    for (i,c) in S.chars().enumerate(){
        let screen_char = WRITER.lock().buffer.chars[BUFFER_HEIGHT -2 ][i].read();
        assert_eq!(char::from(screen_char.ascii_character),c);
    }
}
```

这个测试函数使用`println!`将字符串写入 VGA 字符缓冲区,然后又从`WRITER`中读取字符缓冲区.

## 集成测试

在 Rust 中,**集成测试(integration tests)**的约定是将其放到项目根目录中的`test`目录下.无论是默认测试框架还是自定义测试框架都将自动获取并执行该目录下的所有的测试.

所有的集成测试都是他们自己的可执行文件,并且与我们的`main.rs`完全独立.这也就意味着每个测试都需要定义他们自己的函数入口点.我们创建一个名为`basic_boot`的例子:

```rust
// in tests/basic_boot.rs
#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn _start()->!{
    test_main();
    loop{}
}

fn test_runner(tests:&[&dyn Fn()]){
    unimplemented!();
}

#[panic_handler]
fn panic(info:&PanicInfo)->!{
    loop{}
}
```

这里需要注意的是,集成测试环境中的测试都是单独的可执行文件,所以我们需要再次提供所有的 crate 属性(no_std 等).还需要提供新的入口函数`_start`,用于调用测试入口函数`test_main`.但是我们不需要任何`[cfg]`条件编译属性.

## 创建一个库

为了让这些函数能在我们的集成测试中使用,我们需要从我们的`main.rs`中分割出一个库,这个哭应当可以被其他的 crate 和集成测试可执行文件使用.为了达成这个目的,我们创建了一个新文件,`src/lib.rs`:

```rust
// in src/lib.rs

#![no_std]
```

和`main.rs`一样,`lib.rs`也是一个被 cargo 自动识别的特殊文件.该库是一个独立编译单元,所以我们再次指定`#![no_std]`属性.

为了让我们的库可以和`cargo test`一起协同工作,我们还需要移动一下测试函数和属性:

```rust
#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]
use core::panic::PanicInfo;


//测试函数
pub trait Testable {
    fn run(&self)->();
}

impl<T> Testable for T where T:Fn(),{
    fn run(&self){
        serial_print!("{}...\t",core::any::type_name::<T>());
        self();
        serial_println!("[OK]");
    }
}

pub fn test_runner(tests: &[&dyn Testable]){
    serial_println!("Running {} tests",tests.len());
    for test in tests{
        test.run();
    }
    exit_qemu(QemuExitCode::Success);
}

#[test_case]
fn trivial_assertion(){
    assert_eq!(1,1);
}


#[cfg(test)]
#[no_mangle]
pub extern "C" fn _start() -> !{
    println!("hello world1{}","!");
    test_main();
    loop {
    }
}

#[cfg(test)]
#[panic_handler]
fn panic(_info:&PanicInfo)->!{
    serial_println!("[failed]\n");
    serial_println!("Error: {}",_info);
    exit_qemu(QemuExitCode::Failed);
    loop {
    }
}
```

为了能在可执行文件和集成测试中使用`test_runner`,我们不对其使用`[cfg(test)]`属性,并将其设为 public.同时还将 panic handler 函数分解为 public`test_panic_handler`和`panic`函数.

将`QemuExitCode`枚举和`exit_qemu`函数从`main.rs`移至`lib.rs`中并设为 public:

```rust
//in src/lib.rs
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode){
    use x86_64::instructions::port::Port;
    unsafe{
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}
```

现在,可执行文件和集成测试都可以从苦衷导入这些函数,而不需要自己实现定义.为了使用`println!,serial_println!`可以将一下模块声明也移到`lib.rs`中:

```rust
//in src/lib.rs
pub mod serial;
pub mod vga_buffer;
```

现在修改`main.rs`来使用库:

```rust
#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rust_os::test_runner)]
#![reexport_test_harness_main = "test_main"]
use core::panic::PanicInfo;
use rust_os::println;



#[no_mangle]
pub extern "C" fn _start() -> !{
    println!("hello world1{}","!");
    loop {
    }
}
//panic处理函数
#[cfg(not(test))]
#[panic_handler]
fn panic(_info:&PanicInfo)->!{
    println!("{}",_info);
    loop {
    }
}
```


可以看到,这个库用起来和一个普通的外部库 crate 一样.

## 完成集成测试

此时我们的`tests/basic_boot.rs`就可以使用直接从库中导入各种类型了.

```rust
#![test_runner(rust_os::test_runner)]
use rust_os::test_panic_handler;
```