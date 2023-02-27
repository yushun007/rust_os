# 裸机程序

- 添加#![no_std]属性之后,编译会提示 println!宏为标准库提供,此时无法使用,因此删除此宏调用

- 删除 println!宏之后,cargo biuld 编译会报:

  ```shell
    error: `#[panic_handler]` function required, but not found

    error: language item required, but not found: `eh_personality`
      |
      = note: this can occur when a binary crate with `#![no_std]` is compiled for a target where `eh_personality` is defined in the standard library
      = help: you may be able to compile for a target that doesn't need `eh_personality`, specify a target with `--target` or in `.cargo/config`

    error: could not compile `rust_os` due to 2 previous errors
  ```

  - 此错误表示我们缺少一个#[panic_handler]函数和一个语言选项`eh_personality`
