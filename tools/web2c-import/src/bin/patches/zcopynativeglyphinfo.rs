    pub(crate) unsafe fn zcopynativeglyphinfo(&mut self, src: halfword, dest: halfword) {
        let mut mem: *mut memoryword = self.state.zmem.as_mut_ptr();
        let glyphcount = Self::copy_native_glyph_info(self, src, dest);
        (*mem.offset((dest + 4) as isize)).v.QQQQ.u.B3 = glyphcount;
        (*mem.offset((dest + 5) as isize)).ptr = nullptr;
    }
