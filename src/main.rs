#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(clacos::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![deny(unsafe_op_in_unsafe_fn)]

extern crate alloc;

use core::panic::PanicInfo;

use bootloader::{entry_point, BootInfo};
use x86_64::VirtAddr;

use clacos::{allocator, memory};

mod serial;
mod vga_buffer;

/// entry point
pub fn kernel_main(boot_info: &'static BootInfo) -> ! {
    println!("Hello, {}!", "world");

    clacos::init();

    // init the memory mapper and build a frame allocator using the regions set up by the
    // bootloader
    let physical_memory_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(physical_memory_offset) };
    let mut frame_allocator =
        unsafe { memory::BootInfoFrameAllocator::init(&boot_info.memory_map) };

    // Set up the heap
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");

    #[cfg(test)]
    test_main();

    println!("Did not crash!");
    clacos::hlt_loop();
}
entry_point!(kernel_main); // type-checked way of defining kernel_main as the _start function

/// Panic handler that infinite loops since we're baremetal (regular version)
#[cfg(not(test))]
#[panic_handler]
pub fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    clacos::hlt_loop();
}

/// Testing mode panic handler
#[cfg(test)]
#[panic_handler]
pub fn panic(info: &PanicInfo) -> ! {
    clacos::test_panic_handler(info);
}
