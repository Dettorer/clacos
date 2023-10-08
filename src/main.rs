#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(clacos::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;

mod serial;
mod vga_buffer;

/// entry point (regular version)
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("Hello, {}!", "world");

    loop {}
}

/// entry point (test mode version)
#[cfg(test)]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    test_main();
    loop {}
}

/// Panic handler that infinite loops since we're baremetal (regular version)
#[cfg(not(test))]
#[panic_handler]
pub fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

/// Testing mode panic handler
#[cfg(test)]
#[panic_handler]
pub fn panic(info: &PanicInfo) -> ! {
    clacos::test_panic_handler(info);
}
