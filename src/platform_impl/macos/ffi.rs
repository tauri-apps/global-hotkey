use std::ffi::c_void;

pub type EventRef = *const c_void;

#[link(name = "Carbon", kind = "framework")]
extern "C" {
    pub fn GetEventParameter(
        inEvent: isize,
        inName: isize,
        inDesiredType: isize,
        outActualType: *mut c_void,
        inBufferSize: isize,
        outActualSize: *mut c_void,
        outData: *mut c_void,
    );
}
