#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(unused)]

/* taken from https://github.com/wusyong/carbon-bindgen/blob/467fca5d71047050b632fbdfb41b1f14575a8499/bindings.rs */

pub type UInt32 = ::std::os::raw::c_uint;
pub type SInt32 = ::std::os::raw::c_int;
pub type OSStatus = SInt32;
pub type FourCharCode = UInt32;
pub type OSType = FourCharCode;
pub type ByteCount = ::std::os::raw::c_ulong;
pub type ItemCount = ::std::os::raw::c_ulong;
pub type OptionBits = UInt32;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct OpaqueEventRef {
    _unused: [u8; 0],
}
pub type EventRef = *mut OpaqueEventRef;
pub type EventParamName = OSType;
pub type EventParamType = OSType;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct OpaqueEventHandlerRef {
    _unused: [u8; 0],
}
pub type EventHandlerRef = *mut OpaqueEventHandlerRef;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct OpaqueEventHandlerCallRef {
    _unused: [u8; 0],
}
pub type EventHandlerCallRef = *mut OpaqueEventHandlerCallRef;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct OpaqueEventTargetRef {
    _unused: [u8; 0],
}
pub type EventTargetRef = *mut OpaqueEventTargetRef;
pub type EventHandlerProcPtr = ::std::option::Option<
    unsafe extern "C" fn(
        inHandlerCallRef: EventHandlerCallRef,
        inEvent: EventRef,
        inUserData: *mut ::std::os::raw::c_void,
    ) -> OSStatus,
>;
pub type EventHandlerUPP = EventHandlerProcPtr;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct OpaqueEventHotKeyRef {
    _unused: [u8; 0],
}
pub type EventHotKeyRef = *mut OpaqueEventHotKeyRef;

pub type _bindgen_ty_1637 = ::std::os::raw::c_uint;
pub const kEventParamDirectObject: _bindgen_ty_1637 = 757935405;
pub const kEventParamDragRef: _bindgen_ty_1637 = 1685217639;
pub type _bindgen_ty_1921 = ::std::os::raw::c_uint;
pub const typeEventHotKeyID: _bindgen_ty_1921 = 1751869796;
pub type _bindgen_ty_1939 = ::std::os::raw::c_uint;
pub const kEventClassKeyboard: _bindgen_ty_1939 = 1801812322;
pub type _bindgen_ty_1980 = ::std::os::raw::c_uint;
pub const kEventHotKeyPressed: _bindgen_ty_1980 = 5;
pub type _bindgen_ty_1 = ::std::os::raw::c_uint;
pub const noErr: _bindgen_ty_1 = 0;

#[repr(C, packed(2))]
#[derive(Debug, Copy, Clone)]
pub struct EventHotKeyID {
    pub signature: OSType,
    pub id: UInt32,
}

#[repr(C, packed(2))]
#[derive(Debug, Copy, Clone)]
pub struct EventTypeSpec {
    pub eventClass: OSType,
    pub eventKind: UInt32,
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
        outData: *mut ::std::os::raw::c_void,
    ) -> OSStatus;
    pub fn GetApplicationEventTarget() -> EventTargetRef;
    pub fn InstallEventHandler(
        inTarget: EventTargetRef,
        inHandler: EventHandlerUPP,
        inNumTypes: ItemCount,
        inList: *const EventTypeSpec,
        inUserData: *mut ::std::os::raw::c_void,
        outRef: *mut EventHandlerRef,
    ) -> OSStatus;
    pub fn RemoveEventHandler(inHandlerRef: EventHandlerRef) -> OSStatus;
    pub fn RegisterEventHotKey(
        inHotKeyCode: UInt32,
        inHotKeyModifiers: UInt32,
        inHotKeyID: EventHotKeyID,
        inTarget: EventTargetRef,
        inOptions: OptionBits,
        outRef: *mut EventHotKeyRef,
    ) -> OSStatus;
    pub fn UnregisterEventHotKey(inHotKey: EventHotKeyRef) -> OSStatus;
}
