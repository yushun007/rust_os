# VGA Text Mode

VGA字符模式(VGA text mode)是打印字符到屏幕的一种简单的方式.本章包装此模式为一个安全而简单的接口,我们将包装`unsafe`代码到独立模块.还将实现 rust 语言格式化宏(formatting macros)的支持

## VGA 字符缓冲区

为了在 VGA 字符模式中向屏幕打印字符,我们必须将它写入应将提供的 VGA 字符缓冲区(VGA text buffer).通常情况下,VGA 字符缓冲区是一个 25 行,80 列的二维数组,他的内容将被实时渲染到屏幕.这个数组的元素被称为**字符单元(character cell)**,其使用下面的格式描述一个屏幕上的字符:

|Bit(s)|Value|
|-|-|
|0-7|ASCII code point|
|8-11|Foreground color|
|12-14|Background color|
|15|Blink|

第一个字节表示了应当输出的 ASCII 字节码,第二个字节表示字符的显示方式,前四个表示前景色,后四个表示背景色,最后一个表示是否闪烁.[颜色表](https://os.phil-opp.com/zh-CN/vga-text-mode/)

要修改 VGA 字符缓冲区,我们需要通过**存储器映射输入输出(memory-mapped I/O)**的方式,读取或者写入地址`0xb8000`;这也就意味着我们可以像操作普通内存一样操作这个地址.

需要注意的是,一些硬件虽然映射到存储器,但可能不会完全支持所有的内存操作:可能或有一些设备支持按`u8`字节读取,但在读取`u64`是会返回无效的数据.luckily 字符缓冲区都支持标准的读写操作,我们不需要用特殊的标准对待它.

## 包装到 rust 模块

现在我们创建一个 rust 模块来处理文字打印:

```rust
//in src/main.rs
mod vga_buffer;
```

我们的模块暂时不需要添加子模块,所以我们将它创建在`src/vga_buffer.rs`文件中.

### 颜色

```rust
//in src/vga_buffer.rs

#[allow(dead_code)]
#[derive[Debug,Clone,Copy,PartialEq,Eq]]
#[repr(u8)]
pub enum Color{
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}
```

这里我们使用类似于 C 语言的枚举类(C-like enum),为每个颜色明确一个数字.这里使用`repr(u8)`标记标注的枚举类型,都会以一个`u8`的形式存储--事实上 4 个二进制位就足够了,但是 Rust 没有`u4`类型.

通常来说,编译器会对每个未使用的变量发出**警告(warning)**;使用`#[allow(dead_code)]`属性可以对自定义的数据结构禁用此警告.

`#[derive[Debug,Clone,Copy,PartialEq,Eq]]`为我们**生成(dervie)**生成了`Debug,Clone,Copy,PartialEq,EQ`这几个`trait`(特征):这让我们的类型存寻**复制语义(copy semantic)**,也让其可以被比较,被调试和打印.

为了描述包含前景色和背景色的,完整的**颜色代码(color code)**,我们基于`u8`创建一个新类型:

```rust
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    fn new(foreground:Color , background:Color)->ColorCode{
        ColorCode((background as u8)<<4 | (foreground as u8))
    }
}
```

这里,`ColorCode`类型包转了一个完整的颜色字节码,包含前景色和背景色.和 Color 类型类似,我们为其生成了一系列的`trait`为了确保`ColorCode`和`u8`完全兼容(有相同的内存布局),我们添加`repr(transparent)`标记

### 字符缓冲区

现在,我们可以添加更多的结构体,来描述屏幕上的字节和整个字符缓冲区:

```rust
```