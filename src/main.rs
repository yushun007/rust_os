//core程序需要一个 panic_handler 属性
//自定义 panic_handler 属性
//PanicInfo 中包含了 panic 发生时的文件名,代码行数可选地错误信息.这个函数从不返回所以被定义为***发散函数***.如下所示发散函数的返回值被定义为`never`类型('never'type),记为!
#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]
use core::panic::PanicInfo;
mod vga_buffer;
mod serial;


#[no_mangle]
pub extern "C" fn _start() -> !{
    println!("hello world1{}","!");

    #[cfg(test)]
    test_main();
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
#[cfg(test)]
#[panic_handler]
fn panic(_info:&PanicInfo)->!{
    serial_println!("[failed]\n");
    serial_println!("Error: {}",_info);
    exit_qemu(QemuExitCode::Failed);
    loop {
    }
}

//测试函数
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
    assert_eq!(0,1);
    serial_println!("[OK]");
}

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