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
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
#[repr(C)]
struct ScreenChar{
    ascii_character: u8,
    color_code:ColorCode,
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

#[repr(transparent)]
struct Buffer{
    cahrs:[[ScreenChar;BUFFER_WIDTH];BUFFER_HEIGHT],
}

```

在内存布局层面,rust 并不保证按顺序布局成员变量.因此我们使用`#[repr(C)]`标记结构体;这将按照 C语言约定顺序布局成员变量.对`Buffer`类型,我们咋次使用`repr(transparent)`,确保类型和他的单个成员有相同的内存布局.


为了输出字符到屏幕,创建一个 Writer 类型:

```rust
pub struct Writer{
    column_position:usize,
    color_code:ColorCode,
    buffer:&'static mut Buffer,
}
```

我们将让`Writer`类型将字符写入屏幕的最后一行,并在一行写满或者接受到换行符`\n`的时候,将所有字符向上位移一行.`colum_position`变量将跟踪光标在最后一行的位置.当前字符的颜色由`color_code`表示;另外,我们存入一个Buffer 类型的可变借用到`buffer`变量中.<font color=red>这里我们对借用使用**显式生命周期(explicti lifetime)**,告诉编译器这个借用在何时有效:我们使用`'static`lifetime,这意味着这个借用应该在整个程序的运行期间都有效;这对一个全局有效的 VGA 字符缓冲区来说,是非常合理的</font>

### 打印字符

现在使用`Writer`类型来更改缓冲区内的字符.首先,为了写入一个 ASCII 字节码,创建:

```rust
impl Writer{
    pub fn write_byte(&mut self,byte:u8){
        match byte{
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= BUFFER_WIDTH{
                    self.new_line();
                }
                
                let row = BUFFER_HEIGHT -1;
                let col = self.column_position;
                let color_code = self.color_code;
                self.buffer.cahrs[row][col] = ScreenChar{
                    ascii_character: byte,
                    color_code,
                };
                self.column_position += 1;
            }
        }
    }
    fn new_line(&mut self){
        /* TODO */
    }
}
```

如果这个字节是个换行符`\n`,`Writer`不应该打印新字符,相反会调用稍后实现的`new_line`函数;其他字节将在`match`语句的第二个分支中被打印到屏幕上.

要打印整个字符串,我们把它转换成字节并依次输出:

```rust
    pub fn write_string(&mut self,s:&str){
        for byte in s.bytes(){
            match byte{
                //可以使能够打印的字节码,也可以是'\n'
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                //其他字符打印0xfe
                _ => self.write_byte(0xfe),
            }
        }
    }
```

VGA字符缓冲区只支持 ASCII 字节码和[代码页](https://en.wikipedia.org/wiki/Code_page_437)定义的字符.rust 语言的字符默认编码为`UTF-8`,因此可能包含一些 VGA 不支持的字节码:我们使用`match`语句,来区别可打印和无法打印的字节.无法打印的字符我们打印`0xfe`字符.

为了测试我们的`Writer`我们临时编写一段代码:

```rust
pub fn print_someting(){
    let mut writer = Writer{
        column_position:0,
        color_code:ColorCode::new(Color::Yellow,Color::Black),
        buffer: unsafe {& mut *(0xb8000 as *mut Buffer)},
    };
    writer.write_byte(b'H');
    writer.write_string("ello ");
    writer.write_string("Wörld!");
}
```

这个函数首先创建一个指向`0xb8000`地址 VGA 缓冲区的`Writer`.实现这一点我们需要编写的代码看起来有点奇怪:首先我们把整数`0xb8000`强制转换成**裸指针(raw pointer)**;之后,通过运算符`*`解引用;最后,我们通过`&mut`,再次获得它的可变借用.这些转换需要`unsafe`语句块,因为编译器并不能保证这个裸指针是有效的.

然后它将字节`b'H'`吸入缓冲区.前缀`b`创建了一个**字节常量(byte literal)**,表示单个 ASCII 字节码;通过尝试写入`ello,Wörld!`,测试`write_string`方法.然后我们在`_start`函数中调用`print_someting`方法:

```rust
#[no_mangle]
pub extern "C" fn _start()->!{
    vga_buffer::print_someting();
    loop{}
}
```

### 易失操作

由于 rust 编译器优化功能很激进,我们的代码中的`buffer`只存在写入而没有读取的操作,rust 编译器可能会给优化掉.

这些写入操作应该被认为是易失操作,所以我们需要告诉编译器这些东西不能优化掉.

这里我们使用`volatile`库.这个包(crate)提供了一个名为`Volatile`的**包装类型(wrapping type)**和它的`read,write`方法:这些方法包装了`core::ptr`内的`read_volatile,write_volatile`函数,从而保证读操作或者写操作不会被编译器优化掉.

要添加`volatile`包为项目的依赖项,在`Cargo.toml`文件中添加:

```config
[dependencies]
volatile = "0.2.6"
```

现在,我们使用他来完成 VGA 缓冲区的volatile写入操作.我们将`Buffer`类型定义修改为下列代码:

```rust
```