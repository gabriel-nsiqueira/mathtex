    pub(crate) unsafe fn openorclosein(&mut self) {
        let c_0 = self.state.curchr as eightbits;
        (&mut *(self as *mut PortableTexEngine<'_>)).scanfourbitint();
        let n = self.state.curval as eightbits;
        let slot = n as usize;
        if self.state.readopen[slot] as i32 != 2 as i32 {
            Self::boundary_close_file(self.state.readfile[slot]);
            self.state.readfile[slot] = core::ptr::null_mut();
            self.state.readopen[slot] = 2 as eightbits;
        }
        if c_0 as i32 != 0 {
            (&mut *(self as *mut PortableTexEngine<'_>)).scanoptionalequals();
            (&mut *(self as *mut PortableTexEngine<'_>)).scanfilename();
            (&mut *(self as *mut PortableTexEngine<'_>)).zpackfilename(
                self.state.curname,
                self.state.curarea,
                self.state.curext,
            );
            if Self::boundary_open_input(
                self as *mut PortableTexEngine<'_>,
                &mut self.state.readfile[slot] as *mut unicodefile,
                resource_format_tex_input,
                FOPEN_RBIN_MODE.as_ptr(),
            ) != 0
            {
                self.state.readopen[slot] = 1 as eightbits;
            }
        }
    }
