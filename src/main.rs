#![no_std]
#![no_main]

use core::panic::PanicInfo;

mod vga_buffer;

/// Panic handler that infinite loops since we're baremetal
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("Hello, this is {} Test.", "a");
    panic!("oh no!");

    loop {}
}
