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
use volatile::Volatile;
#[repr(transparent)]
struct Buffer{
    cahrs:[[Volatile<ScreenChar>;BUFFER_WIDTH];BUFFER_HEIGHT],
}
```

这里我们不使用`ScreenChar`,而使用`Volatile<ScreenChar>`,`Volatile`类型是一个泛型,可以包装几乎所有的类型--这保证了我们不会通过普通的写入操作,以外的向他写入数据;我们转而使用提供的`write`方法.

同时我们需要修改`Writer::write_byte`方法:

```rust
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
                self.buffer.cahrs[row][col].write(ScreenChar{
                    ascii_character: byte,
                    color_code,
                });
                self.column_position += 1;
            }
        }
    }
```

这里不再使用普通的`=`赋值操作,而使用`write`方法.

## 格式化宏

支持 Rust 提供的**格式化宏(formatting macros)**也是一种很好的思路.通过这种途径,我们可以轻松的打印不同类型的变量,如整数,浮点数等.为了支持他们,我们需要实现`core::fmt::Write`trait;要实现它, 唯一需要提供的方法是`write_str`,他和我们先前编写的`write_string`方法类似.只是返回值变成了`fmt::Result`:

```rust
use core::fmt;
impl fmt::Write for Writer {
    fn write_str(&mut self,s:&str)->fmt::Result{
        self.write_string(s);
        Ok(())
    }
}
```

这里`Ok(())`属于`Result`枚举类型中的`Ok`,包含一个值为`()`(空值)的变量.

现在我们就可以使用 Rust 内置的格式化宏`write!`和`writeln!`:

```rust
pub fn print_someting(){
    use core::fmt::Write;
    let mut writer = Writer{
        column_position:0,
        color_code:ColorCode::new(Color::Green,Color::Black),
        buffer: unsafe {
            &mut *(0xb8000 as *mut Buffer)
        },
    };
    writer.write_byte(b'H');
    writer.write_string("ello! ");
    write!(writer,"The numbers are {} and {}",23,1.0/3.0).unwrap();
}
```

`write!`宏返回的`Result`类型必须被使用,所以我们调用它的`unwrap`方法,它将在错误发生时 panic.

### 换行

在之前的代码中,我们忽略了换行符,因此没有处理超出一行字符的情况.当换行时,我们想要把每个字符向上移动一行--此时最顶上的一行将被删除--然后在最后一行的起始位置继续打印.要走到这一点,我们需要为`Writer`实现一个新的方法`new_line`:

```rust
    fn new_line(&mut self){
        for row in 1..BUFFER_HEIGHT{
            for col in 0..BUFFER_WIDTH{
                let character = self.buffer.chars[row][col].read();
                self.buffer.chars[row-1][col].write(character);
            }
        }
    }
```

我们遍历屏幕上的每个字符,把每个字符移动到它上方一行的相应位置.这里,`..`符号表示区间标号的一种;它表示左闭右开的区间,因此不包含其上界.在外层的没居中,我们从第一行开始,省略对第 0 行的枚举过程--因为这一行应该被移出屏幕,即它将被下一行的字符覆写.

所以我们实现的`clear_row`方法如下:

```rust
    fn clear_row(&mut self,row:usize){
        let blank = ScreenChar{
            ascii_character:b' ',
            color_code:self.color_code,
        };
        for col in 0..BUFFER_WIDTH{
            self.buffer.chars[row][col].write(blank);
        }
    }
```

通过向对应缓冲区写入空格,清空一行字符.

## 全局接口

编写其他模块时,我们希望无需随时拥有`Write`实例,便能使用它的方法.我们尝试创建一个静态的`WRITER`变量:

```rust
pub static WRITER: Writer = Writer{
    column_position:0,
    color_code:ColorCode::new(Color::Green,Color::Blue),
    buffer: unsafe {
        &mut *(0xb8000 as *mut Buffer)
    },
};
```

此时我们编译会报错:

```shell
   Compiling rust_os v0.1.1 (/Users/yushun/develop/rust_os)
error[E0015]: cannot call non-const fn `ColorCode::new` in statics
   --> src/vga_buffer.rs:131:16
    |
131 |     color_code:ColorCode::new(Color::Green,Color::Blue),
    |                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    |
    = note: calls in statics are limited to constant functions, tuple structs and tuple variants
    = note: consider wrapping this expression in `Lazy::new(|| ...)` from the `once_cell` crate: https://crates.io/crates/once_cell

error[E0658]: dereferencing raw mutable pointers in statics is unstable
   --> src/vga_buffer.rs:133:9
    |
133 |         &mut *(0xb8000 as *mut Buffer)
    |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    |
    = note: see issue #57349 <https://github.com/rust-lang/rust/issues/57349> for more information
    = help: add `#![feature(const_mut_refs)]` to the crate attributes to enable

Some errors have detailed explanations: E0015, E0658.
For more information about an error, try `rustc --explain E0015`.
error: could not compile `rust_os` due to 2 previous errors
```

这里我们需要知道一点:一般变量在运行时初始化,而静态变量则是在编译时初始化.rust 编译器规定了一个成为**常量求解器(const evaluator)**的组件,他会在编译时处理这样的初始化操作.虽然现在其功能有限,但是对他的扩展很活跃,比如允许在常量中 panic 的[一篇文章](https://github.com/rust-lang/rfcs/pull/2345)

关于`ColorCode::new`的问题应该使用`consts functions` 解决,但常量求解器还不能处理在编译时直接将裸指针到变量的引用.

### 延迟初始化

使用非常函数初始化静态变量是 Rust 程序员普遍遇到的问题.幸运的是,有一个叫做`lazy_static`的包提供了一个很棒的解决方案:他提供了一个`lazy_static!`宏,定义一个**延迟初始化(lazily initialized)**的静态变量;这个变量的值将在第一次使用时计算,而非在编译时计算.这时,变量的初始化过程将在运行时进行,任意的初始化代码--无论简单或者复杂--都能够使用.

导入此包:

```config
[dependencies.lazy_static]
version = "1.0"
features = ["spin_no_std"]
```

由于我们不使用标准库所以启用,`spin_no_std`特性.

使用`lazy_static`我们就可以定义一个不出问题的`WRITER`变量:

```rust
use lazy_static::lazy_static;
lazy_static!{
    pub static ref WRITER: Writer = Writer{
        column_position:0,
        color_code:ColorCode::new(Color::Green,Color::Blue),
        buffer: unsafe {
            &mut *(0xb8000 as *mut Buffer)
        },
    };
}
```

然而,这个`WRITER`可能没什么用,因为它目前还是不可变量(immutable variable):
这意味着我们无法向其写入数据.一种方法是使用可变静态变量,但是对其的所有读写操作都被标记为不安全(unsafe)操作,因为这容易造成数据竞争.一些替代方案是使用`RefCell`或者`UnsafeCell`等类型提供的内部可变性;但这些类型都被设计为非同步类型,即不满足 `Sync`约束,所以我们不能再静态变量中使用他们.

## spinlock

要定义同步的内部可变性,我们往往使用标准库提供的**互斥锁类(Mutex)**,他提供当资源被占用时将线程阻塞的互斥条件.实现这一点:我们初步的内核代码还没有线程和阻塞的概念,现在还不能使用这个类.不过我们还有一种较为基础的互斥锁实现方式--**自旋锁(spinlock)**.自旋锁不会调用阻塞逻辑,而是在一个小的无限循环中反复尝试获得这个锁,也因此会一直占用 CPU 时间,知道互斥锁被他的占用着释放.

为了使用自旋锁,我们添加`spin`包:

```config
[dependencies]
spin="0.5.2"
```

现在,我们能够使用自旋锁,为我们的`WRITER`实现安全的内部可变性:

```rust
use spin::Mutex;
lazy_static!{
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position:0,
        color_code:ColorCode::new(Color::Green,Color::Blue),
        buffer: unsafe {
            &mut *(0xb8000 as *mut Buffer)
        },
    });
}
```

现在我们可以删除`print_someting`函数,尝试直接在`_start`函数中直接打印:

```rust
#[no_mangle]
pub extern "C" fn _start() -> !{
    use core::fmt::Write;
    vga_buffer::WRITER.lock().write_str("Hello again").unwrap();
    write!(vga_buffer::WRITER.lock(),",some numbers:{} {}",23,1.0/3.0).unwrap();
    loop{}
}
```

这里导入`core::fmt::Write`tarit,来使用实现它的类的相应方法.

## 安全性

经过上面的努力,我们现在的代码只剩下一个`unsafe`代码块,它用于创建一个指向`0xb8000`地址的`Buffer`类型引用;在这步之后,所有的操作都是安全的.Rust 将检查每个数组的边界,所以我们不会在不经意间越界.因此我们把需要条件编码到 Rust 的类型系统,这之后,我们外界提供的接口就符合内存安全原则了.

## println!宏

我们现在有了一个全局实例`WRITER`,我们就可以基于它实现`println!`宏,这样他就能被任意地方的代码使用了.Rust 提供的宏定义语法需要时间理解,所以我们将不从零开始编写这个宏.

标准库中`println!`的实现如下:

```rust
#[macro_export]
macro_rules! println{
    ()=>(print!("\n"));
    ($($arg:tt)*) => (print!("{}\n",format_args!($($arg)*)));
}
```

<font color=blue>宏通过一个或多个**规则(rule)**定义,有点像`match`语句的多分支.`println!`宏有两个规则:第一个不要求传入参数--`println!()`--他将被扩展为`print!("\n")`,打印一个新行;另一个要求传入参数-- `println!("rust ...")`--它将使用`print!`宏扩展,传入他的所有参数,并在输出的字符串后面加一个换行符.</font>

这里,`#[macro_export]`属性让整个包和基于他的包都能访问这个宏,二不仅限于定义他的模块.他还将把宏至于包的根模块(crate root)下,这意味着我们需要通过`use std::println`导入而不是`use std::macros::println`.

`print!`宏的定义如下:

```rust
#[macro_export]
macro_rules! print{
    ($($arg:tt)*) => ($crate::io::_print(format_args!($($arg)*)));
}
```

这个宏将扩展为一个对 io 模块中`_print`函数的调用.`$crate`变量将在 `std`包之外被解析为`std`包,保证整个宏在`std`包之外可以使用.

`format_args!`宏将传入的参数搭建为一个`fmt::Arguments`类型,这个类型将被传入`_print`函数.`std`包中的`_print`函数将调用复杂的私有函数`print_to`,来处理对不同`Stdout`设备的支持.我们不需要写这样复杂的函数,因为我们只需要支持 VGA 输出.

要打印到字符缓冲区,我们把`println!,print!`两个函数复制过来,修改部分源码,让这些宏使用我们自定义的`_print`函数:

```rust
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::vga_buffer::_print(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! println {
    () => {
        $crate::print!("\n")
    };
    ($($arg::tt)*) => {
        $crate::print!("{}\n",format_args!($($arg)*))
    };
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments){
    use core::fmt::Write;
    WRITER.lock().write_fmt(args).unwrap();
}
```

我们先修改了`println!`宏,在每个`print!`宏前面添加了`$crate`变量.这样我们在只需要使用`println!`时,不必也编写代码导入`print!`宏.

注意我们对两个宏都是用了`#[macro_export]`属性,所以现在这两个宏属于根命名空间(root namespace),所以我们需要使用`use crate::println`导入.

另外,`_print`函数将占用`WRITER`变量的锁,并调用他的`write_fmt`函数.这个方法是从名为`Write`的 trait 中获得的,所以我们需要导入这个 trait.额外的`unwrap()`函数将在打印不成功时 panic.

如果这个宏将在模块外访问,它们也应当能访问`_print`函数,因此这个函数必须是 pub 函数.然而,考虑到这是一个私有实现,这里添加`doc(hidden)`属性,防止其出现在文档中.

### 使用 println!的 Hello World

