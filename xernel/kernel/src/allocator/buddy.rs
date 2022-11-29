// Credits to Stupremee (https://github.com/Stupremee)
// https://github.com/Stupremee/novos/blob/main/crates/kernel/src/allocator/buddy.rs

use super::{align_up, AllocStats, Error, Result};
use crate::{
    dbg,
    mem::{FRAME_SIZE, HIGHER_HALF_OFFSET},
};
use core::{cmp, ptr::NonNull};
use x86_64::VirtAddr;

pub const MAX_ORDER: usize = 12;

pub const MIN_ORDER_SIZE: usize = FRAME_SIZE as usize;

pub fn size_for_order(order: usize) -> usize {
    (1 << order) * MIN_ORDER_SIZE
}

pub fn order_for_size(size: usize) -> usize {
    let size = cmp::max(size, MIN_ORDER_SIZE);
    let size = size.next_power_of_two() / MIN_ORDER_SIZE;
    size.trailing_zeros() as usize
}

fn buddy_of(block: NonNull<usize>, order: usize) -> Result<NonNull<usize>> {
    let buddy = block.as_ptr() as usize ^ size_for_order(order);
    NonNull::new(buddy as *mut _).ok_or(Error::NullPointer)
}

struct ListNode {
    next: Option<NonNull<ListNode>>,
}

pub struct BuddyAllocator {
    orders: [Option<NonNull<ListNode>>; MAX_ORDER],
    pub stats: AllocStats,
}

impl BuddyAllocator {
    /// Create a empty and uninitialized buddy allocator.
    pub const fn new() -> Self {
        Self {
            orders: [None; MAX_ORDER],
            stats: AllocStats::with_name("Physical Memory"),
        }
    }

    pub unsafe fn add_region(&mut self, start: NonNull<u8>, end: NonNull<u8>) -> Result<usize> {
        let start = start.as_ptr();
        let mut start = align_up(start as _, MIN_ORDER_SIZE) as *mut u8;
        let end = end.as_ptr();

        if (end as usize).saturating_sub(start as usize) < MIN_ORDER_SIZE {
            return Err(Error::RegionTooSmall);
        }

        if end < start {
            return Err(Error::InvalidRegion);
        }

        let mut total = 0;
        while (end as usize).saturating_sub(start as usize) >= MIN_ORDER_SIZE {
            let order = self.add_single_region(start, end)?;
            let size = size_for_order(order);

            start = start.add(size);
            total += size;
        }

        Ok(total)
    }

    unsafe fn add_single_region(&mut self, start: *mut u8, end: *mut u8) -> Result<usize> {
        // TODO: Optimize so it doesn't need a loop
        let start_addr = start as usize;

        let mut order = 0;
        while order < (MAX_ORDER - 1) {
            let size = size_for_order(order + 1);

            let new_end = match start_addr.checked_add(size) {
                Some(num) if num <= end as usize => num,
                _ => break,
            };

            let buddy = buddy_of(NonNull::new(start as *mut _).unwrap(), order + 1)?.as_ptr();
            if new_end <= end as usize && (start.cast() <= buddy && buddy <= end.cast()) {
                order += 1;
            } else {
                break;
            }
        }

        // push the block to the list for the given order
        self.order_push(order, NonNull::new(start).unwrap().cast());

        // update statistics
        let size = size_for_order(order);
        self.stats.total += size;
        self.stats.free += size;

        Ok(order)
    }

    pub fn allocate(&mut self, order: usize) -> Result<NonNull<u8>> {
        // check if we exceeded the maximum order
        if order >= MAX_ORDER {
            return Err(Error::OrderTooLarge);
        }

        if let Some(block) = self.order_pop(order) {
            let size = size_for_order(order);
            self.alloc_stats(size);

            return NonNull::new(block.as_ptr().cast()).ok_or(Error::NullPointer);
        }

        let block = self
            .allocate(order + 1)
            .map_err(|_| Error::NoMemoryAvailable)?;

        let buddy = buddy_of(block.cast(), order)?;

        self.order_push(order, buddy.cast());

        let size = size_for_order(order);
        self.alloc_stats(size);

        Ok(block)
    }

    pub unsafe fn deallocate(&mut self, block: NonNull<u8>, order: usize) -> Result<()> {
        let buddy_addr = buddy_of(block.cast(), order)?;

        if self.order_remove(order, buddy_addr.cast()) {
            let size = size_for_order(order);
            self.alloc_stats(size);

            let new_block = cmp::min(buddy_addr.cast(), block);

            let new_order = order + 1;
            if new_order >= MAX_ORDER {
                self.order_push(order, buddy_addr.cast());
                self.order_push(order, block.cast());

                self.dealloc_stats(size * 2);
            } else {
                self.deallocate(new_block, new_order)?;
            }
        } else {
            self.order_push(order, block.cast());

            let size = size_for_order(order);
            self.dealloc_stats(size);
        }

        Ok(())
    }

    fn order_push(&mut self, order: usize, ptr: NonNull<ListNode>) {
        let head = self.orders[order];

        unsafe {
            let vptr = VirtAddr::new(ptr.as_ptr() as u64 + *HIGHER_HALF_OFFSET);
            vptr.as_mut_ptr::<ListNode>().write(ListNode { next: head });
        }

        self.orders[order] = Some(ptr.cast());
    }

    fn order_pop(&mut self, order: usize) -> Option<NonNull<ListNode>> {
        let head = self.orders[order]?;
        let vhead = VirtAddr::new(head.as_ptr() as u64 + *HIGHER_HALF_OFFSET);

        unsafe {
            self.orders[order] = (*vhead.as_ptr::<ListNode>()).next;
        }

        Some(head)
    }

    fn order_remove(&mut self, order: usize, to_remove: NonNull<ListNode>) -> bool {
        let mut cur: *mut Option<NonNull<ListNode>> = match self.orders.get_mut(order) {
            Some(cur) => cur,
            None => return false,
        };

        while let Some(ptr) = unsafe { *cur } {
            dbg!("{:?}", ptr);
            let vptr =
                VirtAddr::new(ptr.as_ptr() as u64 + *HIGHER_HALF_OFFSET).as_mut_ptr::<ListNode>();

            if ptr == to_remove {
                unsafe {
                    *cur = (*vptr).next;
                }
                return true;
            }

            unsafe {
                cur = &mut (*vptr).next;
            }
        }

        false
    }

    fn alloc_stats(&mut self, size: usize) {
        self.stats.free -= size;
        self.stats.allocated += size;
    }

    fn dealloc_stats(&mut self, size: usize) {
        self.stats.free += size;
        self.stats.allocated -= size;
    }
}
