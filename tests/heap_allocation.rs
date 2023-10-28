#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(clacos::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;

use alloc::{boxed::Box, vec::Vec};
use bootloader::{entry_point, BootInfo};
use clacos::{
    allocator::{self, HEAP_SIZE},
    memory::{self, BootInfoFrameAllocator},
};
use x86_64::VirtAddr;

extern crate alloc;

entry_point!(main);

fn main(boot_info: &'static BootInfo) -> ! {
    // initialize the kernel and its memory handling
    clacos::init();
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");

    test_main();
    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    clacos::test_panic_handler(info);
}

/// Check that basic allocations works without error
#[test_case]
fn simple_allocation() {
    let heap_value_1 = Box::new(41);
    let heap_value_2 = Box::new(42);
    assert_eq!(*heap_value_1, 41);
    assert_eq!(*heap_value_2, 42);
}

/// Check big and multiple allocation through a growing vec
#[test_case]
fn large_vec() {
    let n = 1000;
    let mut vec = Vec::new();
    for i in 0..n {
        vec.push(i);
    }
    assert_eq!(vec.iter().sum::<u64>(), (n - 1) * n / 2);
}

/// Check that memory is correctly reclaimed when dropping values
#[test_case]
fn many_boxes() {
    // do HEAP_SIZE Box allocations, which would overflow the heap if the boxes' memory wasn't
    // reclaimed after each loop iteration
    for i in 0..HEAP_SIZE {
        let x = Box::new(i);
        assert_eq!(*x, i);
    }
}
