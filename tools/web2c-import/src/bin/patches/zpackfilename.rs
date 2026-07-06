    pub(crate) unsafe fn zpackfilename(&mut self, n: strnumber, a: strnumber, e: strnumber) {
        self.nameoffile_storage.clear();
        self.nameoffile_storage.push(0);
        let mut append_pool_range = |engine: &mut PortableTexEngine<'_>, start: poolpointer, end: poolpointer| {
            if start > end {
                return;
            }
            let mut units = Vec::with_capacity((end - start + 1).max(0) as usize);
            let mut cursor = start;
            loop {
                let c = *engine.state.strpool.offset(cursor as isize);
                if c as i32 != 34 as i32 {
                    units.push(c);
                }
                let previous = cursor;
                cursor += 1;
                if previous >= end {
                    break;
                }
            }
            let mut encoded = [0; 4];
            for codepoint in char::decode_utf16(units) {
                let codepoint = codepoint.unwrap_or(char::REPLACEMENT_CHARACTER);
                for byte in codepoint.encode_utf8(&mut encoded).as_bytes() {
                    engine.nameoffile_storage.push(*byte as UTF8code);
                }
            }
        };
        if let Some(area_index) = Self::pool_string_index(a) {
            let area_start = *self.state.strstart.offset(area_index);
            let area_end = (*self.state.strstart.offset(area_index + 1) as i32 - 1) as poolpointer;
            append_pool_range(self, area_start, area_end);
        }
        if let Some(name_index) = Self::pool_string_index(n) {
            let name_start = *self.state.strstart.offset(name_index);
            let name_end = (*self.state.strstart.offset(name_index + 1) as i32 - 1) as poolpointer;
            append_pool_range(self, name_start, name_end);
        }
        if let Some(ext_index) = Self::pool_string_index(e) {
            let ext_start = *self.state.strstart.offset(ext_index);
            let ext_end = (*self.state.strstart.offset(ext_index + 1) as i32 - 1) as poolpointer;
            append_pool_range(self, ext_start, ext_end);
        }
        let namelength = self.nameoffile_storage.len().saturating_sub(1).min(maxint as usize);
        self.state.namelength = namelength as integer;
        self.nameoffile_storage.truncate(namelength + 1);
        self.nameoffile_storage.push(0);
        self.state.nameoffile = self.nameoffile_storage.as_mut_ptr();
    }
