//core程序需要一个 panic_handler 属性
//自定义 panic_handler 属性
//PanicInfo 中包含了 panic 发生时的文件名,代码行数可选地错误信息.这个函数从不返回所以被定义为***发散函数***.如下所示发散函数的返回值被定义为`never`类型('never'type),记为!
#![no_std]
#![no_main]
use core::panic::PanicInfo;
static HELLO: &[u8] = b"hello world!";

#[no_mangle]
pub extern "C" fn _start() -> !{
    let vga_buffer = 0xb8000 as *mut u8;
    for (i,&byte) in HELLO.iter().enumerate(){
        unsafe {
            *vga_buffer.offset(i as isize * 2) = byte;
            *vga_buffer.offset(i as isize * 2 + 1) = 0xb;
        }
    }
    loop{}
}
//裸机程序,因为没有操作系统的支持,所以不能使用标准库,需要添加#![no_std]属性
#[panic_handler]
fn panic(_info:&PanicInfo)->!{
    loop{}
}