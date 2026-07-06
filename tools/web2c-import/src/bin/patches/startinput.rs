    pub(crate) unsafe fn startinput(&mut self) {
        let eqtb = self.state.zeqtb.as_mut_ptr();
        (&mut *(self as *mut PortableTexEngine<'_>)).scanfilename();
        (&mut *(self as *mut PortableTexEngine<'_>)).zpackfilename(
            self.state.curname,
            self.state.curarea,
            self.state.curext,
        );
        (&mut *(self as *mut PortableTexEngine<'_>)).beginfilereading();
        if Self::boundary_open_input(
            self as *mut PortableTexEngine<'_>,
            self.state.inputfile.offset(self.state.curinput.indexfield as isize) as *mut unicodefile,
            resource_format_tex_input,
            FOPEN_RBIN_MODE.as_ptr(),
        ) == 0
        {
            (&mut *(self as *mut PortableTexEngine<'_>)).endfilereading();
            return;
        }
        self.state.curinput.namefield = self.state.curname as halfword;
        *self
            .state
            .sourcefilenamestack
            .offset(self.state.inopen as isize) = self.state.curname;
        *self
            .state
            .fullsourcefilenamestack
            .offset(self.state.inopen as isize) = self.state.curname;
        self.state.curinput.statefield = 33 as quarterword;
        self.state.line = 1 as integer;
        let _ = Self::boundary_input_line(
            self as *mut PortableTexEngine<'_>,
            *self.state.inputfile.offset(self.state.curinput.indexfield as isize) as NativeFileHandle,
        ) != 0;
        (&mut *(self as *mut PortableTexEngine<'_>)).firmuptheline();
        let endline_char_index = if self.is_xetex() && self.state.eqtbtop >= 7_892_312 {
            7_892_312_i64
        } else {
            27_212_i64
        };
        let endline_char = if eqtb.is_null() {
            -1
        } else {
            (*eqtb.offset(endline_char_index as isize)).u.CINT
        };
        if !(0..=255).contains(&endline_char) {
            self.state.curinput.limitfield -= 1;
        } else {
            *self
                .state
                .buffer
                .offset(self.state.curinput.limitfield as isize) = endline_char as UnicodeScalar;
        }
        self.state.first = (self.state.curinput.limitfield as i32 + 1) as integer;
        self.state.curinput.locfield = self.state.curinput.startfield;
    }
