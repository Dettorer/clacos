#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(clacos::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![deny(unsafe_op_in_unsafe_fn)]

use core::panic::PanicInfo;

use bootloader::{entry_point, BootInfo};

mod serial;
mod vga_buffer;

/// entry point
pub fn kernel_main(boot_info: &'static BootInfo) -> ! {
    println!("Hello, {}!", "world");

    clacos::init();

    // XXX: virtual memory allocation demonstration
    use clacos::memory;
    use x86_64::{
        structures::paging::{Mapper, Page, PageTableFlags, PhysFrame},
        PhysAddr, VirtAddr,
    };
    // init the mapper and build a frame allocator using the regions set up by the bootloader
    let physical_memory_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(physical_memory_offset) };
    let mut frame_allocator =
        unsafe { memory::BootInfoFrameAllocator::init(&boot_info.memory_map) };

    // the page we want to map in virtual space
    let page: Page = Page::containing_address(VirtAddr::new(0xdeadbeaf000));

    // the frame we want to map it to (some place in the middle of the screen's memory region)
    let frame = PhysFrame::containing_address(PhysAddr::new(0xb8000));
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

    // do the mapping using our frame allocator
    let map_to_result = unsafe { mapper.map_to(page, frame, flags, &mut frame_allocator) };
    map_to_result.expect("map_to failed").flush();

    // write to the page and see if it appears on screen
    let page_ptr: *mut u64 = page.start_address().as_mut_ptr();
    unsafe { page_ptr.offset(400).write_volatile(0x_f021_f077_f065_f04e) };
    // XXX: end of demonstration

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
