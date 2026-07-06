    pub(crate) unsafe fn znewnativecharacter(
        &mut self,
        f_0: internalfontnumber,
        c_0: UnicodeScalar,
    ) -> halfword {
        let mut mem: *mut memoryword = self.state.zmem.as_mut_ptr();
        let mut eqtb: *mut memoryword = self.state.zeqtb.as_mut_ptr();
        if (*eqtb.offset(7892299 as i64 as isize)).u.CINT > 0 as i32
            && Self::map_char_to_glyph(
                self as *mut PortableTexEngine<'_>,
                f_0 as integer,
                c_0 as integer,
            ) == 0 as i32
        {
            (&mut *(self as *mut PortableTexEngine<'_>)).zcharwarning(f_0, c_0);
        }

        let p = (&mut *(self as *mut PortableTexEngine<'_>)).zgetnode(7 as i32);
        (*mem.offset(p as isize)).hh.u.B0 = 8 as i16;
        (*mem.offset(p as isize)).hh.u.B1 = 40 as i16;
        (*mem.offset((p as i32 + 4 as i32) as isize)).v.QQQQ.u.B0 = 7 as quarterword;
        (*mem.offset((p as i32 + 4 as i32) as isize)).v.QQQQ.u.B1 = f_0 as quarterword;
        (*mem.offset((p as i32 + 4 as i32) as isize)).v.QQQQ.u.B3 = 0 as quarterword;
        (*mem.offset((p as i32 + 5 as i32) as isize)).ptr = nullptr as voidpointer;
        if c_0 as i64 > 65535 as i64 {
            (*mem.offset((p as i32 + 4 as i32) as isize)).v.QQQQ.u.B2 = 2 as quarterword;
            *(mem.offset((p as i32 + native_node_size) as isize) as *mut memoryword as *mut u16)
                .offset(0 as i32 as isize) =
                ((c_0 as i64 - 65536 as i64) / 1024 as i64 + 55296 as i64) as u16;
            *(mem.offset((p as i32 + native_node_size) as isize) as *mut memoryword as *mut u16)
                .offset(1 as i32 as isize) =
                ((c_0 as i64 - 65536 as i64) % 1024 as i64 + 56320 as i64) as u16;
        } else {
            (*mem.offset((p as i32 + 4 as i32) as isize)).v.QQQQ.u.B2 = 1 as quarterword;
            *(mem.offset((p as i32 + native_node_size) as isize) as *mut memoryword as *mut u16)
                .offset(0 as i32 as isize) = c_0 as u16;
        }
        Self::measure_native_node(
            self as *mut PortableTexEngine<'resources>,
            mem.offset(p as isize) as *mut memoryword as voidpointer,
            ((*eqtb.offset(7892342 as i64 as isize)).u.CINT > 0 as i32) as i32,
        );
        p
    }
