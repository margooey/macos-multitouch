extern crate core_foundation_sys;
extern crate libc;

use libc::*;

#[repr(C)]
pub struct MtPoint {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
pub struct MtReadout {
    pub pos: MtPoint,
    pub vel: MtPoint,
}

#[repr(C)]
pub struct Finger {
    pub frame: i32,
    pub timestamp: f64,
    pub identifier: i32,
    pub state: i32,
    pub finger_number: i32,
    pub palm_rejection: i32, // 0 if palm, ±1 if finger (generally)
    pub normalized: MtReadout,
    pub size: f32,
    pub pressure: i32,   // see https://github.com/KrishKrosh/TrackWeight
    pub angle: f32,      // \
    pub major_axis: f32, //  |- ellipsoid
    pub minor_axis: f32, // /
    pub mm: MtReadout,
    pub unknown2: [i32; 2],
    pub capacitance: f32, // how strong the electrical signal is for a touch, lower = weaker
}

pub type MTDeviceRef = *const c_void;

#[allow(clippy::duplicated_attributes)]
#[link(name = "MultitouchSupport", kind = "framework")]
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    //MTDeviceRef MTDeviceCreateDefault();
    pub fn MTRegisterContactFrameCallbackWithRefcon(
        device: MTDeviceRef,
        callback: extern "C" fn(MTDeviceRef, &Finger, c_int, c_double, c_int, *mut c_void) -> c_int,
        user_data: *mut c_void,
    ) -> c_void;

    pub fn MTDeviceStart(device: MTDeviceRef, number: i32) -> c_void; // thanks comex
    pub fn MTDeviceStop(device: MTDeviceRef, number: i32) -> c_void;
    pub fn MTDeviceCreateList() -> core_foundation_sys::array::CFArrayRef; //returns a CFMutableArrayRef array of all multitouch devices
}

extern "C" fn callback_handler(
    device: MTDeviceRef,
    data: &Finger,
    length: c_int,
    timestamp: c_double,
    frame: c_int,
    user_data: *mut c_void,
) -> c_int {
    #[allow(clippy::type_complexity)]
    let closure: &mut &mut dyn FnMut(MTDeviceRef, &[Finger], f64, i32) = unsafe {
        &mut *(user_data
            as *mut &mut dyn for<'a> std::ops::FnMut(*const libc::c_void, &'a [Finger], f64, i32))
    };
    let fingers = unsafe { std::slice::from_raw_parts(data, length as usize) };
    closure(device, fingers, timestamp, frame);

    0 as c_int
}

pub struct MultitouchDevice {
    _device: MTDeviceRef,
    is_started: bool,
}

impl MultitouchDevice {
    fn new(device: MTDeviceRef) -> MultitouchDevice {
        MultitouchDevice {
            _device: device,
            is_started: false,
        }
    }

    pub fn register_contact_frame_callback<F>(&mut self, callback: F) -> Result<(), &'static str>
    where
        F: FnMut(MTDeviceRef, &[Finger], f64, i32),
    {
        if !self.is_started {
            #[allow(clippy::type_complexity)]
            let cb: Box<Box<dyn FnMut(MTDeviceRef, &[Finger], f64, i32)>> =
                Box::new(Box::new(callback));
            unsafe {
                MTRegisterContactFrameCallbackWithRefcon(
                    self._device,
                    callback_handler,
                    Box::into_raw(cb) as *mut _,
                );
            }
            self.is_started = true;
            unsafe { MTDeviceStart(self._device, 0) };
            return Ok(());
        }

        Err("There is already a callback registered to this device.")
    }

    pub fn stop(&mut self) {
        unsafe { MTDeviceStop(self._device, 0) };
    }
}

pub fn get_multitouch_devices() -> Vec<MultitouchDevice> {
    let device_list = unsafe { MTDeviceCreateList() };
    let count = unsafe { core_foundation_sys::array::CFArrayGetCount(device_list) };

    let mut ret_val: Vec<MultitouchDevice> = Vec::new();
    for i in 0..count {
        ret_val.push(MultitouchDevice::new(unsafe {
            core_foundation_sys::array::CFArrayGetValueAtIndex(device_list, i)
        }));
    }

    ret_val
}
