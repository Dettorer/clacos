#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(clacos::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(allocator_api)]

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

/// Check that memory is correctly reclaimed when dropping values even in the presence of
/// long-lived data
#[test_case]
fn many_boxes_with_long_lived() {
    // first allocate a value that is meant to live through the entire function
    let long_lived = Box::new(-1);

    // do HEAP_SIZE Box allocations, which would overflow the heap if the boxes' memory wasn't
    // reclaimed after each loop iteration
    for i in 0..HEAP_SIZE {
        let x = Box::new(i);
        assert_eq!(*x, i);
    }

    // check that the first allocated value is still there
    assert_eq!(*long_lived, -1);
}

/// Check that memory is reclaimed in a way that does not overly fragment free region
#[test_case]
fn many_long_lived_small_then_big() {
    // We will fill the heap with relatively small allocations, then drop the values, then try to
    // reuse the space by allocating one large value.
    const MEDIUM_SIZE: usize = HEAP_SIZE / 16;
    const LARGE_SIZE: usize = HEAP_SIZE / 8;
    struct MediumData {
        data: [u8; MEDIUM_SIZE],
    }
    struct LargeData {
        data: [u8; LARGE_SIZE],
    }

    // First allocate enough medium-sized objects to fill at least half of the heap
    let medium_size_count: usize = 13;
    let mut boxes: Vec<Box<MediumData>> = Vec::with_capacity(medium_size_count);
    for i in 0..medium_size_count {
        let new_data = Box::try_new(MediumData {
            data: [0; MEDIUM_SIZE],
        })
        .unwrap_or_else(|msg| {
            panic!(
                "Could not allocate a medium data at iteration {}/{}: {}",
                i, medium_size_count, msg
            )
        });
        boxes.push(new_data);
    }
    // Check their values to make sure they are used
    for b in boxes.iter() {
        assert_eq!(b.data[0], 0);
    }

    // Reclaim memory
    drop(boxes);

    // Allocate enough large-sized data to fill at least half of the heap
    let large_size_count: usize = 7;
    let mut boxes: Vec<Box<LargeData>> = Vec::with_capacity(large_size_count);
    for i in 0..large_size_count {
        let new_data = Box::try_new(LargeData {
            data: [0; LARGE_SIZE],
        })
        .unwrap_or_else(|msg| {
            panic!(
                "Could not allocate a large data at iteration {}/{}: {}",
                i, large_size_count, msg
            )
        });
        boxes.push(new_data);
    }
    // Check their values to make sure they are used
    for b in boxes.iter() {
        assert_eq!(b.data[0], 0);
    }
}
