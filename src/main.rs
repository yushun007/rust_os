//core程序需要一个 panic_handler 属性
//自定义 panic_handler 属性
//PanicInfo 中包含了 panic 发生时的文件名,代码行数可选地错误信息.这个函数从不返回所以被定义为***发散函数***.如下所示发散函数的返回值被定义为`never`类型('never'type),记为!
#![no_std]
#![no_main]
use core::panic::PanicInfo;
mod vga_buffer;

#[no_mangle]
pub extern "C" fn _start() -> !{
    use core::fmt::Write;
    vga_buffer::WRITER.lock().write_str("Hello again").unwrap();
    write!(vga_buffer::WRITER.lock(),",some numbers:{} {}",23,1.0/3.0).unwrap();
    loop{}
}
//裸机程序,因为没有操作系统的支持,所以不能使用标准库,需要添加#![no_std]属性
#[panic_handler]
fn panic(_info:&PanicInfo)->!{
    loop{}
}