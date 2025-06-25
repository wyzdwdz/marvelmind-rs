// Copyright 2025 wyzdwdz <wyzdwdz@gmail.com>
//
// Licensed under the MIT license <LICENSE or https://opensource.org/licenses/MIT>.
// This file may not be copied, modified, or distributed except according to
// those terms.

//! Marvelmind<sup>&copy;</sup> api wrapper
//! 
//! # Example
//! 
//! ```rust
//! use marvelmind as mm;
//! 
//! let version = mm::api_version().unwrap();
//! println!("api version: {}", version);
//! 
//! mm::open_port(30).unwrap();
//! println!("open port successfully");
//! 
//! let mut devices_list = mm::get_device_list().unwrap();
//! let _ = devices_list.update_last_locations().unwrap();
//! 
//! let devices = device_list.devices();
//! for device in devices {
//!     println!(
//!         "address #{:0>3} x {:.3} y {:.3} z {:.3} q {}",
//!         device.address(),
//!         device.x() as f64 / 1000.0,
//!         device.y() as f64 / 1000.0,
//!         device.z() as f64 / 1000.0,
//!         device.q()
//!     );
//! }
//! ```

use std::{
    fmt, mem,
    thread::sleep,
    time::{self, Instant, SystemTime},
};
use zerocopy::{
    byteorder::little_endian::{I32, U16, U32},
    FromBytes,
};
use zerocopy_derive::{FromBytes, Immutable, KnownLayout, Unaligned};

#[cfg_attr(target_os = "windows", link(name = "dashapi", kind = "raw-dylib"))]
#[cfg_attr(not(target_os = "windows"), link(name = "dashapi"))]
unsafe extern "C" {
    fn mm_get_last_error(pdata: *mut U32) -> bool;
    fn mm_api_version(pdata: *mut U32) -> bool;
    fn mm_open_port() -> bool;
    fn mm_close_port() -> bool;
    fn mm_get_devices_list(pdata: *mut [u8; mem::size_of::<MMDeviceList>()]) -> bool;
    fn mm_get_last_locations2(pdata: *mut [u8; mem::size_of::<MMLastLocations>()]) -> bool;
}

/// Marvelmind<sup>&copy;</sup> api call error
#[derive(Debug, Clone)]
pub enum MMError {
    /// Communication error
    CommunicationError,
    /// Error opening serial port
    SerialPortError,
    /// License is required
    LicenseError,
    /// Unknown error type
    UnknownError,
}

impl fmt::Display for MMError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::CommunicationError => write!(f, "communication error"),
            Self::SerialPortError => write!(f, "error opening serial port",),
            Self::LicenseError => write!(f, "license is required"),
            Self::UnknownError => write!(f, "unknown error"),
        }
    }
}

#[repr(C)]
#[derive(Debug, FromBytes, KnownLayout, Immutable, Unaligned)]
struct MMDeviceList {
    num: u8,
    devices: [MMDevice; u8::MAX as usize + 1],
}

#[repr(C)]
#[derive(Debug, FromBytes, Immutable, Unaligned)]
struct MMDevice {
    address: u8,
    is_duplicated: u8,
    is_sleeping: u8,
    v_major: u8,
    v_minor: u8,
    v_second: u8,
    type_id: u8,
    _firmware_option: u8,
    flags: u8,
}

#[repr(C)]
#[derive(Debug, FromBytes, KnownLayout, Immutable, Unaligned)]
struct MMLastLocations {
    coordinates: [MMCoordinate; 6],
    _is_new: u8,
    _tbd: [u8; 5],
    _size_payload: u8,
    _payload: [u8; u8::MAX as usize + 1],
}

#[repr(C)]
#[derive(Debug, FromBytes, Immutable, Unaligned)]
struct MMCoordinate {
    address: u8,
    _head_index: u8,
    x: I32,
    y: I32,
    z: I32,
    _status_flag: u8,
    q: u8,
    _tbd0: u8,
    _tbd1: u8,
    _tbd2: U16,
}

/// Marvelmind<sup>&copy;</sup> devices list
#[derive(Debug, Clone)]
pub struct DeviceList {
    devices: Vec<Device>,
}

impl DeviceList {
    /// Get Marvelmind<sup>&copy;</sup> devices information.
    #[inline]
    pub fn devices(&self) -> &Vec<Device> {
        &self.devices
    }

    /// Update the last locations of each Marvelmind<sup>&copy;</sup> device.
    /// 
    /// If one of locations is updated, return `true`; otherwise, return `false`.
    pub fn update_last_locations(&mut self) -> Result<bool, MMError> {
        let mut pdata = [0 as u8; mem::size_of::<MMLastLocations>()];
        let update_time = SystemTime::now();
        let res = unsafe { mm_get_last_locations2(&mut pdata) };

        if res == false {
            return Err(get_last_error());
        }

        let mut is_update = false;

        let last_locations = MMLastLocations::ref_from_bytes(&pdata).unwrap();

        for device in &mut self.devices {
            let coord = &last_locations.coordinates;
            for idx in 0..coord.len() {
                if coord[idx].address == device.address && coord[idx].q <= 100 {
                    device.x = coord[idx].x.into();
                    device.y = coord[idx].y.into();
                    device.z = coord[idx].z.into();
                    device.q = coord[idx].q;
                    device.update_time = update_time;
                    is_update = true;
                }
            }
        }

        Ok(is_update)
    }
}

/// The information of Marvelmind<sup>&copy;</sup> device
#[derive(Debug, Clone)]
pub struct Device {
    address: u8,
    is_duplicated: bool,
    is_sleeping: bool,
    v_major: u8,
    v_minor: u8,
    v_second: u8,
    dtype: DeviceType,
    is_connected: bool,
    x: i32,
    y: i32,
    z: i32,
    q: u8,
    update_time: SystemTime,
}

impl Device {
    /// Get the address of Marvelmind<sup>&copy;</sup> device.
    #[inline]
    pub fn address(&self) -> u8 {
        self.address
    }

    /// If the address of device is duplicated - more than 1 device with same address was found.
    #[inline]
    pub fn is_duplicated(&self) -> bool {
        self.is_duplicated
    }

    /// If the device is sleeping.
    #[inline]
    pub fn is_sleeping(&self) -> bool {
        self.is_sleeping
    }

    /// Get major version of firmware (example: "6", for version V6.07a).
    #[inline]
    pub fn v_major(&self) -> u8 {
        self.v_major
    }

    /// Get minor version of firmware (example: "7", for version V6.07a).
    #[inline]
    pub fn v_minor(&self) -> u8 {
        self.v_minor
    }

    /// Get second minor version of firmware (example: "1", for version V6.07a).
    #[inline]
    pub fn v_second(&self) -> u8 {
        self.v_second
    }

    /// Get Marvelmind<sup>&copy;</sup> device type.
    #[inline]
    pub fn dtype(&self) -> DeviceType {
        self.dtype.clone()
    }

    /// If the device has confirmed connection.
    #[inline]
    pub fn is_connected(&self) -> bool {
        self.is_connected
    }

    /// Get X coordinate, mm.
    #[inline]
    pub fn x(&self) -> i32 {
        self.x
    }

    /// Get Y coordinate, mm.
    #[inline]
    pub fn y(&self) -> i32 {
        self.y
    }

    /// Get Z coordinate, mm.
    #[inline]
    pub fn z(&self) -> i32 {
        self.z
    }

    /// Get Quality of positioning, 0...100%.
    #[inline]
    pub fn q(&self) -> u8 {
        self.q
    }

    /// Get the time information when updating location of the device.
    #[inline]
    pub fn update_time(&self) -> SystemTime {
        self.update_time
    }
}

/// Marvelmind<sup>&copy;</sup> device type
#[derive(Debug, Clone)]
pub enum DeviceType {
    /// Beacon HW V4.5
    BeaconHwV45,
    /// Beacon HW V4.5 (hedgehog mode)
    BeaconHwV45Hedgehog,
    /// Modem HW V4.9
    ModemHwV49,
    /// Beacon HW V4.9
    BeaconHwV49,
    /// Beacon HW V4.9 (hedgehog mode)
    BeaconHwV49Hedgehog,
    /// Beacon Mini-RX
    BeaconMiniRx,
    /// Beacon Mini-TX
    BeaconMiniTx,
    /// Beacon-TX-IP67
    BeaconTxIp67,
    /// Beacon industrial-RX
    BeaconIndustrialRx,
    /// Super-Beacon
    SuperBeacon,
    /// Super-Beacon (hedgehog mode)
    SuperBeaconHedgedog,
    /// Industrial Super-Beacon
    IndustrialSuperBeacon,
    /// Industrial Super-Beacon (hedgehog mode)
    IndustrialSuperBeaconHedgedog,
    /// Super-Modem
    SuperModem,
    /// Modem HW V5.1
    ModemHwV51,
}

impl TryFrom<u8> for DeviceType {
    type Error = &'static str;

    fn try_from(id: u8) -> Result<Self, Self::Error> {
        match id {
            22 => Ok(Self::BeaconHwV45),
            23 => Ok(Self::BeaconHwV45Hedgehog),
            24 => Ok(Self::ModemHwV49),
            30 => Ok(Self::BeaconHwV49),
            31 => Ok(Self::BeaconHwV49Hedgehog),
            32 => Ok(Self::BeaconMiniRx),
            36 => Ok(Self::BeaconMiniTx),
            37 => Ok(Self::BeaconTxIp67),
            41 => Ok(Self::BeaconIndustrialRx),
            42 => Ok(Self::SuperBeacon),
            43 => Ok(Self::SuperBeaconHedgedog),
            44 => Ok(Self::IndustrialSuperBeacon),
            45 => Ok(Self::IndustrialSuperBeaconHedgedog),
            46 => Ok(Self::SuperModem),
            48 => Ok(Self::ModemHwV51),
            _ => Err("Unspecific device type id"),
        }
    }
}

fn get_last_error() -> MMError {
    let mut err_id: U32 = U32::ZERO;
    let res = unsafe { mm_get_last_error(&mut err_id) };

    match res {
        true => match u32::from(err_id) {
            1 => MMError::CommunicationError,
            2 => MMError::SerialPortError,
            3 => MMError::LicenseError,
            _ => MMError::UnknownError,
        },
        false => MMError::UnknownError,
    }
}

/// Reads version of the API library. Required to ensure the needed functions are available in this version of library.
pub fn api_version() -> Result<u32, MMError> {
    let mut version: U32 = U32::ZERO;
    let res = unsafe { mm_api_version(&mut version) };

    match res {
        true => Ok(version.into()),
        false => Err(get_last_error()),
    }
}

/// Opens port where Marvelmind<sup>&copy;</sup> device (modem or beacon) is connected via USB (virtual serial port). 
/// You don’t need to specify serial port name, because the API searching all serial ports and checks whether it corresponds to Marvelmind device or no.
/// 
/// # Arguments
/// * `timeout` - Maximum wait time in seconds before aborting. 
///   Note: A value of 0 will attempt exactly one opening attempt.
pub fn open_port(timeout: u64) -> Result<(), MMError> {
    let t_start = Instant::now();
    loop {
        if t_start.elapsed().as_secs() > timeout {
            return Err(get_last_error());
        }

        let res = unsafe { mm_open_port() };

        match res {
            true => break,
            false => match res {
                true => break,
                false => sleep(time::Duration::from_millis(1)),
            },
        }
    }

    Ok(())
}

/// Closes port, if it was previously opened by `open_port` function.
pub fn close_port() -> Result<(), MMError> {
    let res = unsafe { mm_close_port() };

    match res {
        true => Ok(()),
        false => Err(get_last_error()),
    }
}

/// Reads list of Marvelmind<sup>&copy;</sup> devices known to modem. 
/// The list includes list of all devices connected by radio to modem’s network, including sleeping devices.
pub fn get_device_list() -> Result<DeviceList, MMError> {
    let mut pdata = [0 as u8; mem::size_of::<MMDeviceList>()];
    let res = unsafe { mm_get_devices_list(&mut pdata) };

    if res == false {
        return Err(get_last_error());
    }

    let device_list = MMDeviceList::ref_from_bytes(&pdata).unwrap();

    let mut devices = Vec::<Device>::new();
    let update_time = SystemTime::now();

    for idx in 0..device_list.num as usize {
        let mmdevice = &device_list.devices[idx];

        let device = Device {
            address: mmdevice.address,
            is_duplicated: mmdevice.is_duplicated != 0,
            is_sleeping: mmdevice.is_sleeping != 0,
            v_major: mmdevice.v_major,
            v_minor: mmdevice.v_minor,
            v_second: mmdevice.v_second,
            dtype: DeviceType::try_from(mmdevice.type_id)
                .unwrap_or_else(|_| panic!("unsupported device type id: {}", mmdevice.type_id)),
            is_connected: mmdevice.flags & 0b00000001 > 0,
            x: 0,
            y: 0,
            z: 0,
            q: 0,
            update_time: update_time,
        };

        devices.push(device);
    }

    Ok(DeviceList { devices: devices })
}
