    pub(crate) unsafe fn ztrienode(&mut self, mut p: triepointer) -> triepointer {
        let mut Result: triepointer = 0;
        let mut h: triepointer = 0;
        let mut q: triepointer = 0;
        // `trie_node`'s hash: abs(c + 1009*o + 2718*l + 3142*r) mod trie_size.
        // The multiplications overflow i32 for large pattern sets (e.g. 3142 * a
        // ~1M trie pointer), in C / release Rust this wraps and the `mod` keeps it
        // a valid hash. Use explicit wrapping ops so debug builds match release
        // byte-for-byte instead of panicking on the overflow check.
        let key = (*self.state.triec.offset(p as isize) as triepointer)
            .wrapping_add(
                (1009 as triepointer)
                    .wrapping_mul(*self.state.trieo.offset(p as isize) as triepointer),
            )
            .wrapping_add(
                (2718 as triepointer).wrapping_mul(*self.state.triel.offset(p as isize)),
            )
            .wrapping_add(
                (3142 as triepointer).wrapping_mul(*self.state.trier.offset(p as isize)),
            );
        h = ((if key >= 0 as i32 { key } else { key.wrapping_neg() }) % self.state.triesize)
            as triepointer;
        loop {
            q = *self.state.triehash.offset(h as isize);
            if q == 0 as i32 {
                *self.state.triehash.offset(h as isize) = p;
                Result = p;
                return Result;
            }
            if *self.state.triec.offset(q as isize) as i32
                == *self.state.triec.offset(p as isize) as i32
                && *self.state.trieo.offset(q as isize) as i32
                    == *self.state.trieo.offset(p as isize) as i32
                && *self.state.triel.offset(q as isize) == *self.state.triel.offset(p as isize)
                && *self.state.trier.offset(q as isize) == *self.state.trier.offset(p as isize)
            {
                Result = q;
                return Result;
            }
            if h > 0 as i32 {
                h -= 1;
            } else {
                h = self.state.triesize as triepointer;
            }
        }
    }
