    pub(crate) unsafe fn scanpdfexttoks(&mut self) {
        Self::record_stripped_pdf_extension(self as *mut PortableTexEngine<'_>);
        let _ = (&mut *(self as *mut PortableTexEngine<'_>)).zscantoks(0 as i32, 1 as i32) != 0 as i32;
    }
