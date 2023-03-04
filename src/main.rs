//core程序需要一个 panic_handler 属性
//自定义 panic_handler 属性
//PanicInfo 中包含了 panic 发生时的文件名,代码行数可选地错误信息.这个函数从不返回所以被定义为***发散函数***.如下所示发散函数的返回值被定义为`never`类型('never'type),记为!
#![no_std]
#![no_main]
use core::panic::PanicInfo;
mod vga_buffer;

#[no_mangle]
pub extern "C" fn _start() -> !{
    println!("hello world1{}","!");
    panic!("Some panic message!");
    loop{}
}
//panic处理函数
#[panic_handler]
fn panic(_info:&PanicInfo)->!{
    println!("{}",_info);
    loop{}
}