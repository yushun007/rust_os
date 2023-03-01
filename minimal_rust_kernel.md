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


