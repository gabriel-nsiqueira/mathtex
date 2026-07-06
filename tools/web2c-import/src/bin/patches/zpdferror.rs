    pub(crate) unsafe fn zpdferror(&mut self, _t: strnumber, _p: strnumber) {
        Self::record_stripped_pdf_extension(self as *mut PortableTexEngine<'_>);
    }
