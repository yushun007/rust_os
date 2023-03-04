# 最小内核

## 引导启动

电脑上电时,主板会加载 ROM 内存中存储的固件:其负责电脑的硬件自检(包括可用内存检测,cpu 和其他硬件的预加载),之后会寻找一个可引导的存储介质(bootable disk),并开始引导启动其中的内核.

x86 支持两种固件标准:BIOS(basic Input/Output system)和 UEFI(Unified Extensible Firmware Interface).

## BIOS 启动

由于 BIOS 标准实在太老了,为了兼容 BIOS,系统在启动前必须进入一个 16 位的系统兼容的实模式(real mode),这样一些老机器才能启动.

当电脑上电之后,存储在主板上 ROM 闪存中的 BIOS 固件将被加载.BIOS 固件将会加点自检,初始化硬件,然后它将寻找一个可引导的存储介质.如果找到了,电脑的控制权将移交引导程序(bootloader):一段存储在存储介质的开头的,512 字节长度的程序片段.大多数引导程序长于 512 字节--所以通常会将引导分为多个阶段:例如存储在硬盘开头不超过 512 字节的优先启动的<font color = green>第一阶段引导程序</font>(first stage bootloader)和一段随后由第一段加载的长度较长存储在其他位置的<font color = green>第二段引导程序</font>

引导程序必须决定内核的位置,并将其加载到内存中.引导程序还需要将 CPU 从 16 位实模式切换到 32 的保护模式,并最终切换到 64 位长模式<font color = green>(long mode)</font>:此时所有的 64 位寄存器和整个主内存才能被访问.引导程序的第三个作用,是从 BIOS 查询特定的信息,并将其传递到内核;例如查询和传递内存映射表<font color = green>(memory map)</font>

由于编写一个 bootloader 并不简单,需要使用汇编语言,而且必须经过许多意图不明的步骤--比如把某个幻数写入某个寄存器.这里推荐一个工具 <font color = green>bootiamge 工具</font>--其会自动并且方便的为我们的内核准备一个引导程序

## multiboot标准

每个操作系统都实现了自己的引导程序,而其只对单个系统有效.为了避免这种僵局,95 年自由软件基金会(Free Software Foundation)发布了一个开源的引导程序标准--<font color = green>Multiboot</font>.此标准定义了引导程序和操作系统间的统一接口,所以任何适配<font color = green>Multiboot</font>的引导程序,都能用来加载任何同样适配了 Multiboot 的操作系统.GNU GRUB 是一个可供参考的 Multiboot 实现.

要编写一个适配 Multiboot 的内核,我们只需要在内核文件开头,插入被称作<font color = green>Multiboot 头(Multiboot header)</font>的数据片段.这让 GRUB 很容易引导任何操作系统,但是,GRUB 和 Multiboot 标准也有些可预知的问题:
1. 他们支持 32 保护模式.也就是说引导之后,仍然需要配置 CPU,让其切换到 64 位长模式
2. 他们被设计为精简引导程序,而不是精简内核.举个例子,内核需要已调整过的<font color = green>默认页长度(default page size)</font>被链接,否则 GRUB 将无法找到内核的 Multiboot 头.另外<font color = green>引导信息(boot information)</font>,这个包含着大量与架构有关的数据,会在引导启动时,被直接传到操作系统,而不会经过一层清晰地抽象.
3. GRUB 和 Multiboot 标准并没有被详细的解释,阅读相关文档需要一定的经验.
4. 为了创造一个能够被引导的磁盘镜像,我们在发誓必须安装 GRUB:加到了基于 windows 和 macOS 开发内核的难度.

## 最小内核

上一章我们使用 cargo 构建了一个不依赖任何其他库的独立小程序;但是此程序仍然基于特定的操作系统平台:不同的平台需要定义不同的函数,且编译指令也不尽相同.这是因为默认情况下,rust 会为宿主机构建源码.但是我们需要我们的内核不依赖任何其他的操作系统--我们的目标是编译为一个特定的目标系统(target system).

### 安装 Nightly Rust 版本
由于我们需要使用一些在 Nightly 版本中才有的功能,所以我们需要将 rust 编译器换成 nightly 版本.
`rustup override add nightly`命令可以将当前目录的 rust 编译器设置为 nightly 版本.或者在项目根目录中添加一个名为`rust-toolchain`内容为`nightly`的文件.

### 目标配置清单

通过`--target`参数,cargo 支持不同的目标系统.这个目标系统可以使用一个目标<font color = green>三元组(target triple)</font>来表示,表述了 CPU架构,平台供应商,操作系统和应用程序<font color = green>二进制接口(Applecation Binary Interface,ABI)</font>

为了编写我们的目标系统,并且鉴于我们需要一些特殊的配置(比如没有依赖的底层操作系统),现有的目标三元组都不能满足我们的需求.luckily 只需要一个 JSON 文件,rust 遍允许我们自定义个自己的目标系统:这个文件被称为<font color = green>目标配置清单(target specification)</font>比如,一个描述`x86_64-unknown-linux-gun`目标系统的配置清单如下所示:

```json
{
    "llvm-target":"x86_64-unknown-linux-gun",
    "data-layout":"e-m:e-i64:64-f80:128-n8:16:32:64:64-S128",
    "arch":"x86_64",
    "target-endian":"little",
    "target-pointer-width":"64",
    "target-c-int-width":"32",
    "os":"linux",
    "executables":true,
    "linker-flavor":"gcc",
    "pre-link-args":["-m64"],
    "morestack":false
}
```

一个配置清单中包含多个配置项(field).大多数配置项都是 LLVM 需求的,它们将配置为塔顶平台生成的代码.例如:`data-layout`定义了不同的整数,浮点数,指针类型的长度;还有一些 Rust 用作条件变异的配置项,如`target-pointer-width`.还有一些类型的配置项,定义了这个包应该如何被编译,例如,`pre-link-args`配置项制定了应该向连接器传入的参数.

我们的系统将会编译到`x86_64`架构,所以我们的配置清单和上面的相似.现在创建一个名为`x86_64-glog_os.json`的文件(名字无所谓):

```json
{
    "llvm-target":"x86_64-unknown-none",
    "data-layout":"e-m:e-i64:64-f80:128-n8:16:32:64-S128",
    "arch":"x86_64",
    "target-endian":"little",
    "target-pointer-width":"64",
    "target-c-int-width":"32",
    "os":"none",
    "executable":true,
    "linker-flavor":"ld.lld",
    "linker":"rust-lld",
    "panic-strategy":"abort",
    "disable-redzone":true,
    "features":"-mmx,-sse,+soft-float"
}
```

- `llvm-target,os`项:因为我们是在裸机上运行,所以我们修改`llvm-target,os`项为 none
- `linker-flavor`:由于我们需要不依赖平台的链接器这里选用LLVM 提供的跨平台的连接器<font color = green>LLD 连接器(LLD linker)</font>,这是和 rust 编译器一起打包发布的.
- `panic-strategy`:上一章我们解决`eh-personality`编译报错的时候使用过,我们的编译目标不支持 panic 的时候栈展开,所以我们在 panic 的时候直接终止.这一项是可选的,只要`Cargo.toml`和这里设置一次即可.
- `disable-redzone`:我们正在写一个内核,所以我们迟早要处理中断.要安全的实现这一点,我们必须禁用一个与<font color = red>红区(redzone)</font>有关的栈指针优化:以为这个优化可能会导致栈被破坏.[详细资料](https://os.phil-opp.com/zh-CN/red-zone/)
- `features`:配置项用来启用和禁用某些 CPU 特性.`-`表示禁用,`+`表示启用,这里禁用了`mmx,sse`指令集,因为这两个指令集是SIMD 指令集,主要用来执行并行数据运算.如果内核包含这些指令集,内核将不得不保存庞大的 SIMD 寄存器,这对内核性能影响较大.但是这里又引入了一个问题,就是x86_64架构的浮点运算依赖 SIMD 寄存器.这里我们启用`soft-float`特性.这会使用基于整数的软件模拟浮点运算

### 编译内核

此时我们已经配置好了我们的内核编译选项,这里我们使用 linux 风格的编写风格即,入口函数命名为`_start`.

使用`cargo build --target x86_64-blog_os.json`命令编译内核.

此时会编译报错:

```shell
error[E0463]: can't find crate for `core`
  |
  = note: the `x86_64-blog_os` target may not be installed
  = help: consider downloading the target with `rustup target add x86_64-blog_os`
  = help: consider building the standard library from source with `cargo build -Zbuild-std`

error[E0463]: can't find crate for `compiler_builtins`

error[E0463]: can't find crate for `core`
 --> src/main.rs:6:5
  |
6 | use core::panic::PanicInfo;
  |     ^^^^ can't find crate
  |
  = note: the `x86_64-blog_os` target may not be installed
  = help: consider downloading the target with `rustup target add x86_64-blog_os`
  = help: consider building the standard library from source with `cargo build -Zbuild-std`

error: requires `sized` lang_item

For more information about this error, try `rustc --explain E0463`.
warning: `rust_os` (bin "rust_os") generated 1 warning
error: could not compile `rust_os` due to 4 previous errors; 1 warning emitted
```

错误信息显示`x86_64-blog_os`未安装,core 模块找不到.其中 core 模块中包含了 rust 的部分基础类型如:`Result,Option,迭代器`等等,并且其还隐式的链接到`no_std`特性中.

通常情况下,`core`crate 以<font color = green> 预编译库(precompiled library)</font>的形式与 Rust 编译器一同发布,因此此时`core`只对宿主系统有效,而对我们的自定义系统无效.如果我们想为其他系统编译代码,我们需要为这个系统重新编译整个`core`crate.

`build-std`选项:
cargo 中<font color = green>`build-std`特性</font>,允许我们按照自己的需要重新编译`core`等标准 crate,而不需要使用 Rust 安装程序内置的预编译版本.

要启用该特性,需要在`.cargo/config.toml`文件中添加一个配置:

```cargo
[unstatble]
build-std=["core","compiler_buildins"]
```

此配置告诉 cargo 需要重新编译 `core`和`compiler_buildins`两个 crate,其中`compiler_buildins`是`core`的必要依赖.另外编译需要源码,使用`rustup component add rust-src`命令下载它们.

设置并下载完源码之后我们就可以编译了.

### 内存相关的函数
目前 rust 编译器嘉定所有<font color = green>内置函数(built-in functions)</font> 在所有操作系统内都存在且可用.但是这里只对了一半,绝大多数内置函数都可以被`compiler_buitins`crate提供,而且我们重新编译过了这个 crate,但是部分内存相关的函数需要操作系统相关的标准 C 库提供.比如`memset,memcpy,memcmp`.现在我们还无法提供相关的标准 C 库,所以我们需要其他方法提供这些东西,一种是自己实现相关函数,(注意不要忘记加`#[no_mangle]`,防止编译器重命名).另外一种是使用`compiler_builtins`crate 中提供的相关函数的实现,默认情况下为了避免和标准 C 库发生冲突被禁掉了.可以在`.cargo/config.toml`文件中添加`build-std-features=["compiler-builtins-mem]`项启用这些内置函数.

### 设置默认编译目标

和上一章针对不同平台设置编译目标一样,我们在`.cargo/config.toml`文件中指定编译目标.

```shell
[build]
target="x86_64-blog_os.json"
```

### 向屏幕打印字符

此时我们的内核已经配置完毕,并且能够成功编译`_start`函数,但是此函数仍然是个空循环啥也没有,我们需要向屏幕打印一些东西.

要做到这一步,最简单的方式是写入<font color = green> VGA 字符缓冲区(VGA text buffer)</font>:这是一段映射到 VGA 赢家你的特殊内存片段,包含着显示在屏幕上的内容.通常其能够存储 25 行,80 列共 2000 个字符单元;每个字符单元能够显示一个 ASCII 字符,也能设置字符的前景和背景色.下一章详细讨论 VGA 字符缓冲区的内存布局;这里我们只需知道缓冲区地址是`0x8000`,且每个字符单元包含一个 ASCII 字符字节码和一个颜色字节码.

```rust
static HELLO: &[u8] = b"hello world!";

#[no_mangle]
pub extern "C" fn _start() -> !{
    let vag_buffer = 0xb8000 as *mut u8;
    for (i,&byte) in HELLO.iter().enumerate(){
        unsafe {
            *vga_buffer.offset(i as isize * 2) = byte;
            *vga_buffer.offset(i as isize * 2 + 1) = 0xb;
        }
    }
    loop{}
}
```

这里我们与定义了一个<font color = green>字节字符串(byte string)</font>类型的静态变量,名为`HELLO`.我们首先将整数`0xb8000`转换(cast)成一个裸指针(raw pointer),之后我们迭代`HELLO`每个字节,使用`enumerate`获得一个额外的序号变量`i`.在`for`循环中,我们使用`offset`偏移裸指针,解引用它,来讲字符串的每个字节和对应的颜色--`0xb`淡青色--写入内存位置.

<font color = red>所有裸指针操作都被一个叫`unsafe`的语句块(unsafe block)包裹.</font>因为此时编译器不能保证我们创建的裸指针是有效的:一个裸指针可能只想任何一个你想让其指向的地方,直接解引用并写入它,可能会损坏正常的数据.使用`unsafe`语句块时,程序员告诉编译器,这块代码我负责,你不用管.但是其实`unsafe`并不会关闭 rust 的安全检查机制,其只允许你多做[四件事](https://doc.rust-lang.org/1.30.0/book/second-edition/ch19-01-unsafe-rust.html#unsafe-superpowers):

- 取消引用原始指针
- 调用不安全的函数或者方法
- 访问或修改可变静态变量
- 实时不安全特征

## 启动内核

既然我们已经有了一个可以编译成功,并且可以向屏幕打印字符的程序,我们是时候将他运行起来了.首先我们将编译完毕的内核与引导程序链接,来创建一个引导镜像;之后可以使用 QEMU 虚拟机启动或者使用 U盘在物理机上启动.

### 创建引导镜像

要将可执行程序转换为<font color = green>可引导镜像(bootable disk image)</font>,我们需要将其与引导程序链接.引导程序将负责初始化 CPU 并加载我们的内核.

这里我们使用已有的 bootloader 包;无需依赖 C 语言,这个包基于 rust 代码和内联汇编,实现了一个五脏俱全的 BIOS 引导程序.为了用起启动我们的内核,我们需要将他添加为一个依赖项,`Caro.toml`中添加(这里的版本号不重要,rust 会帮我们检查版本号):

```cargo
[dependencies]
bootloader = "0.9.23"
```

只添加引导程序依赖,并不足以创建一个可引导的镜像;我们还需要内核编译完成之后,将内核引导程序组合在一起.cargo 目前尚不支持编译完成之后添加其他步骤.

为了解决这个问题,建议使用`bootimage`工具--他会在内核编译完毕之后,将它和引导程序组合在一起,最终创建一个能够引导的磁盘镜像.
`cargo install bootimage`命令可以安装此工具.

为了运行`bootimage`以及编译引导程序,还需要安装`rustup`模块`llvm-tools-preview`

完成安装之后运行命令`cargo bootimage`

`bootimage`工具干了三件事:
- 编译我们的内核为一个 ELF 文件
- 编译引导程序位独立的可执行文件
- 将内核ELF 文件按字节拼接到引导程序的末尾

当机器启动时,引导程序将会读取并解析拼接在其后的 ELF 文件.这之后,他讲吧程序片段映射到分页表中的虚拟地址,清零 BSS 段,并创建一个栈.最终它将读取入口地址(即`_start`函数的位置)并跳转到这里.

在 QEMU 中启动内核

现在我们可以再虚拟机中启动内核了.`qemu-system-x86_64 -drive format=raw,file=target/x86_64-blog_os/debug/bootimage-blog_os.bin`