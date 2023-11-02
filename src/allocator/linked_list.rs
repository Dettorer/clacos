//! A heap memory allocator that uses a linked list of unused memory region
//!
//! Main drawbacks:
//!
//! - slow allocation if the heap is very fragmented
//! - free regions are split when used for a small allocation, but never merged back when
//! deallocated again, which can lead to situations where there is more than enough memory
//! available for a caller's request, but fragmented in many regions, none of which are
//! individually large enough to fulfill it.
//!
//! The last drawback could be mitigated (with further performance costs), the first one is
//! inherent to the linked list design.

use core::{mem, ptr};

use alloc::alloc::{GlobalAlloc, Layout};

use super::{fast_align_up, Locked};

/// The metadata stored at the start of every *free* memory region.
///
/// The allocation process finds a suitable memory region and removes it from the list altogether,
/// so these metadata are not needed anymore once the region is allocated and can be safely
/// overwritten by the caller. This is possible because the deallocation function requires the
/// caller to provide the size of the region, which means that we don't have to keep track of it
/// ourselves for used regions, which means that we don't need *any* metadata at all for used
/// regions. It does mean, though, that we need to ensure every allocated region is large enough to
/// store a `ListNode`, because we will write a new one at its start once it gets deallocated (see
/// `LinkedListAllocator::size_align()`).
struct ListNode {
    size: usize,
    next: Option<&'static mut ListNode>,
}

impl ListNode {
    const fn new(size: usize) -> Self {
        ListNode { size, next: None }
    }

    fn start_addr(&self) -> usize {
        self as *const Self as usize
    }

    fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }
}

pub struct LinkedListAllocator {
    head: ListNode,
}

impl LinkedListAllocator {
    /// Create a new LinkedListAllocator initialized as empty with no backing heap area
    pub const fn new() -> LinkedListAllocator {
        Self {
            head: ListNode::new(0),
        }
    }

    /// Initialize the allocator with the given heap bounds.
    ///
    /// # Safety
    ///
    /// This function is unsafe because the caller must guarantee that the given heap bounds are
    /// valid and that the heap is unused. This method must be called only once.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        unsafe {
            self.add_free_region(heap_start, heap_size);
        }
    }

    /// Add the given memory region to the front of the list
    ///
    /// # Safety
    ///
    /// `addr` and `size` must describe a valid memory area that is readable, writeable, and not
    /// currently in use. Using only information given by allocation functions from this module
    /// should suffice.
    unsafe fn add_free_region(&mut self, addr: usize, size: usize) {
        // ensure that the freed region is capable of holding a ListNode struct (check that `addr`
        // is correctly aligned for `ListNode` and that size is large enough to store it).
        // These are asserts because the given `addr` and `size` should have been computed by
        // allocation functions from this same module, so these are effectively consistency checks
        assert_eq!(fast_align_up(addr, mem::align_of::<ListNode>()), addr);
        assert!(size >= mem::size_of::<ListNode>());

        // create a new node and prepend it at the start of our list
        let mut node = ListNode::new(size);
        node.next = self.head.next.take();
        let node_ptr = addr as *mut ListNode;
        unsafe {
            node_ptr.write(node);
            self.head.next = Some(&mut *node_ptr)
        }
    }

    /// Look for a free region compatible with the required size and alignment and remove it from
    /// the list.
    ///
    /// Return a tuple of the list node and the start address of the allocation
    fn find_region(&mut self, size: usize, align: usize) -> Option<(&'static mut ListNode, usize)> {
        // At each iteration we may have three references to three nodes, like this:
        //      previous -> region -> next
        //
        // The node being checked for compatibility is always `region`. Since `self.head` is a
        // dummy first element of size 0, it is a perfect initial value for `previous`.
        let mut previous = &mut self.head;
        while let Some(ref mut region) = previous.next {
            if let Ok(alloc_start) = Self::alloc_from_region_possible(region, size, align) {
                let next = region.next.take();
                let ret = Some((previous.next.take().unwrap(), alloc_start));
                previous.next = next;
                return ret;
            } else {
                previous = previous.next.as_mut().unwrap();
            }
        }

        None
    }

    /// Check if the given memory region is suitable for an allocation of the given size and
    /// alignment.
    ///
    /// On succes, return the alligned start address of the allocation, otherwise return Err(())
    fn alloc_from_region_possible(
        region: &ListNode,
        size: usize,
        align: usize,
    ) -> Result<usize, ()> {
        let alloc_start = fast_align_up(region.start_addr(), align);
        let alloc_end = alloc_start.checked_add(size).ok_or(())?;

        if alloc_end > region.end_addr() {
            // region too small
            return Err(());
        }

        let excess_size = region.end_addr() - alloc_end;
        if excess_size > 0 && excess_size < mem::size_of::<ListNode>() {
            // The rest of the region (the part unused by the allocation) is too small to hold a
            // ListNode. This is needed because, with an non-zero excess size, the region would be
            // split in two by the allocation to form a new free region with the unused part, which
            // needs to be handled by a ListNode.
            return Err(());
        }

        Ok(alloc_start)
    }

    /// Adjust the given layout so that the resulting allocated memory region is also suitable for
    /// storing a `ListNode` (which it will once it is deallocated).
    ///
    /// Return the adjusted size and alignement as a (size, align) tuple.
    fn size_align(layout: Layout) -> (usize, usize) {
        let layout = layout
            .align_to(mem::align_of::<ListNode>())
            .expect("adjusting alignment failed")
            // pad the size to a multiple of the new alignment to make sure the following region's
            // start address has the same alignment
            .pad_to_align();

        // increase the allocation size if needed so that it can at leas hold a `ListNode`
        // XXX: shouldn't it be done *before* using `pad_to_align`? since only the later ensures
        // that `size` is a multiple of the layout's alignment.
        let size = layout.size().max(mem::size_of::<ListNode>());
        (size, layout.align())
    }
}

unsafe impl GlobalAlloc for Locked<LinkedListAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let (size, align) = LinkedListAllocator::size_align(layout);
        let mut allocator = self.lock();

        if let Some((region, alloc_start)) = allocator.find_region(size, align) {
            let alloc_end = alloc_start.checked_add(size).expect("overflow");
            let excess_size = region.end_addr() - alloc_end;
            if excess_size > 0 {
                unsafe {
                    allocator.add_free_region(alloc_end, excess_size);
                }
            }
            alloc_start as *mut u8
        } else {
            // no suitable memory region found (out of memory)
            ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let (size, _) = LinkedListAllocator::size_align(layout);
        let mut allocator = self.lock();
        unsafe {
            allocator.add_free_region(ptr as usize, size)
        }
    }
}
