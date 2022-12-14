#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(unused)]

use std::ffi::{c_ulong, c_void, CStr};

pub type EventRef = *const c_void;
pub type OSStatus = i32;
pub type OSType = u32;
pub type EventParamName = OSType;
pub type EventParamType = OSType;
pub type ByteCount = c_ulong;
pub type EventHandlerRef = *const c_void;
pub type EventHandlerCallRef = *const c_void;
pub type EventTargetRef = *const c_void;
pub type EventHandlerUPP =
    unsafe extern "C" fn(EventHandlerCallRef, EventRef, *mut c_void) -> OSStatus;
pub type ItemCount = c_ulong;
pub type EventHotKeyRef = *const c_void;
pub type OptionBits = u32;

// NOTE: these `&str` consts translation may be wrong
// maybe use `const_cstr` crate to define them?
pub const kEventParamDirectObject: &str = "----";
pub const kEventParamDragRef: &str = "drag";
pub const typeEventHotKeyID: &str = "hkid";
pub const kEventClassKeyboard: &str = "keyb";
pub const kEventHotKeyPressed: u32 = 5;
pub const noErr: i32 = 0;

#[repr(C)]
pub struct EventHotKeyID {
    pub signature: OSType,
    pub id: u32,
}

#[repr(C)]
pub struct EventTypeSpec {
    pub eventClass: OSType,
    pub eventKind: u32,
}

#[link(name = "Carbon", kind = "framework")]
extern "C" {
    pub fn GetEventParameter(
        inEvent: EventRef,
        inName: EventParamName,
        inDesiredType: EventParamType,
        outActualType: *mut EventParamType,
        inBufferSize: ByteCount,
        outActualSize: *mut ByteCount,
        outData: *mut c_void,
    ) -> OSStatus;
    pub fn GetApplicationEventTarget() -> EventTargetRef;
    pub fn InstallEventHandler(
        inTarget: EventTargetRef,
        inHandler: EventHandlerUPP,
        inNumTypes: ItemCount,
        inList: *const EventTypeSpec,
        inUserData: *mut c_void,
        outRef: *mut EventHandlerRef,
    ) -> OSStatus;
    pub fn RemoveEventHandler(inHandlerRef: EventHandlerRef) -> OSStatus;
    pub fn RegisterEventHotKey(
        inHotKeyCode: u32,
        inHotKeyModifiers: u32,
        inHotKeyID: EventHotKeyID,
        inTarget: EventTargetRef,
        inOptions: OptionBits,
        outRef: EventHotKeyRef,
    ) -> OSStatus;
    pub fn UnregisterEventHotKey(inHotKey: EventHotKeyRef) -> OSStatus;
}
