use std::{collections::BTreeSet, mem};

#[derive(Debug, PartialEq, Eq)]
pub struct Block {
    start: u32,
    size: u32,
}
impl Block {
    #[inline(always)]
    fn continues_to(&self, right: &Block) -> bool {
        self.start + self.size == right.start
    }
}

pub struct Allocator {
    free: Vec<Block>,
}

impl Allocator {
    pub fn new(size: u32) -> Self {
        Self {
            free: vec![Block { start: 0, size }],
        }
    }

    pub fn allocate(&mut self, size: u32) -> Option<Block> {
        let size = size.max(4).next_power_of_two();

        let (idx, block) = self
            .free
            .iter_mut()
            .enumerate()
            .filter(|(_, v)| v.size >= size)
            .min_by_key(|(_, v)| v.size - size)?;

        if block.size == size {
            Some(self.free.remove(idx))
        } else {
            // Split off
            let start = block.start;
            *block = Block {
                start: block.start + size,
                size: block.size - size,
            };

            eprintln!("Split off {size}, leaving {block:?}");

            Some(Block { start, size })
        }
    }

    pub fn deallocate(&mut self, block: Block) {
        eprintln!("deallocate {block:?}");
        if self.free.is_empty() {
            self.free.push(block);
            return;
        }

        let idx = self
            .free
            .binary_search_by_key(&block.start, |v| v.start)
            .expect_err("Block is not in free list");
        dbg!(idx);

        if idx == 0 {
            // merge right
            let r = &mut self.free[0];
            if block.continues_to(r) {
                eprintln!("Merging right");
                r.start -= block.size;
                assert_eq!(r.start, block.start);
                r.size += block.size;
            } else {
                self.free.insert(0, block);
            }
        } else if let [l, r] = &mut self.free[idx - 1..=idx] {
            eprintln!("Merge left right");
            if l.continues_to(&block) && block.continues_to(r) {
                eprintln!("Merge left and right");
                l.size += block.size + r.size;
                self.free.remove(idx);
            } else if l.continues_to(&block) {
                l.size += block.size;
            } else if block.continues_to(r) {
                r.start -= block.size;
            } else {
                self.free.insert(idx, block);
            }
        } else {
            eprintln!("Last {idx}");
            assert_eq!(idx, self.free.len());
            assert_ne!(idx, 0);

            let l = &mut self.free[idx];

            if l.continues_to(&block) {
                l.size += block.size;
            } else {
                self.free.insert(idx, block);
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_alloc() {
        let mut allocator = Allocator::new(128);

        let b1 = allocator.allocate(4).unwrap();
        assert_eq!(b1, Block { start: 0, size: 4 });
        let b2 = allocator.allocate(8).unwrap();
        assert_eq!(b2, Block { start: 4, size: 8 });

        assert_eq!(
            allocator.free,
            [Block {
                start: 12,
                size: 116
            }]
        );

        allocator.deallocate(b2);
        assert_eq!(
            allocator.free,
            [Block {
                start: 4,
                size: 124
            }]
        );
        allocator.deallocate(b1);
        assert_eq!(
            allocator.free,
            [Block {
                start: 0,
                size: 128
            }]
        );
    }

    #[test]
    fn test_alloc_mid() {
        let mut allocator = Allocator::new(128);

        let b0 = allocator.allocate(8).unwrap();
        let b1 = allocator.allocate(4).unwrap();
        assert_eq!(b1, Block { start: 8, size: 4 });

        let b2 = allocator.allocate(32).unwrap();
        assert_eq!(
            b2,
            Block {
                start: 12,
                size: 32
            }
        );

        let b3 = allocator.allocate(6).unwrap();
        assert_eq!(b3, Block { start: 44, size: 8 });

        assert_eq!(
            allocator.free,
            [Block {
                start: 52,
                size: 76
            }]
        );

        allocator.deallocate(b3);
        assert_eq!(
            allocator.free,
            [Block {
                start: 44,
                size: 84
            }]
        );

        allocator.deallocate(b1);
        assert_eq!(
            allocator.free,
            [
                Block { start: 8, size: 4 },
                Block {
                    start: 44,
                    size: 84
                }
            ]
        );

        allocator.deallocate(b2);

        assert_eq!(
            allocator.free,
            [Block {
                start: 8,
                size: 120
            },]
        );

        allocator.deallocate(b0);

        assert_eq!(
            allocator.free,
            [Block {
                start: 0,
                size: 128
            },]
        );
    }
}
