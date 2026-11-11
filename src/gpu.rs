//! GPU load and video memory from Windows performance counters (PDH), plus
//! battery state. The same counters Task Manager uses; no admin rights needed.

#[cfg(windows)]
mod imp {
    use windows_sys::Win32::System::Performance::{
        PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PdhAddEnglishCounterW, PdhCloseQuery,
        PdhCollectQueryData, PdhGetFormattedCounterArrayW, PdhOpenQueryW,
    };

    type H = *mut core::ffi::c_void;

    pub struct Gpu {
        query: H,
        util: H,
        mem: H,
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain([0]).collect()
    }

    impl Gpu {
        pub fn new() -> Option<Gpu> {
            // SAFETY: PDH handles are owned by this struct and closed in Drop.
            unsafe {
                let mut query: H = std::ptr::null_mut();
                if PdhOpenQueryW(std::ptr::null(), 0, &mut query) != 0 {
                    return None;
                }
                let (mut util, mut mem): (H, H) = (std::ptr::null_mut(), std::ptr::null_mut());
                let a = PdhAddEnglishCounterW(
                    query,
                    wide(r"\GPU Engine(*engtype_3D)\Utilization Percentage").as_ptr(),
                    0,
                    &mut util,
                );
                let b = PdhAddEnglishCounterW(
                    query,
                    wide(r"\GPU Adapter Memory(*)\Dedicated Usage").as_ptr(),
                    0,
                    &mut mem,
                );
                if a != 0 && b != 0 {
                    PdhCloseQuery(query);
                    return None;
                }
                PdhCollectQueryData(query);
                Some(Gpu {
                    query,
                    util: if a == 0 { util } else { std::ptr::null_mut() },
                    mem: if b == 0 { mem } else { std::ptr::null_mut() },
                })
            }
        }

        fn sum(&self, counter: H) -> Option<f64> {
            if counter.is_null() {
                return None;
            }
            // SAFETY: two-call pattern: ask for the buffer size, then fill it.
            unsafe {
                let (mut size, mut count) = (0u32, 0u32);
                PdhGetFormattedCounterArrayW(
                    counter,
                    PDH_FMT_DOUBLE,
                    &mut size,
                    &mut count,
                    std::ptr::null_mut(),
                );
                if size == 0 {
                    return Some(0.0);
                }
                let mut buf = vec![0u8; size as usize];
                let items = buf.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W;
                if PdhGetFormattedCounterArrayW(
                    counter,
                    PDH_FMT_DOUBLE,
                    &mut size,
                    &mut count,
                    items,
                ) != 0
                {
                    return None;
                }
                let list = std::slice::from_raw_parts(items, count as usize);
                Some(
                    list.iter()
                        .map(|i| i.FmtValue.Anonymous.doubleValue)
                        .filter(|v| v.is_finite())
                        .sum(),
                )
            }
        }

        /// (3D engine load in %, dedicated video memory in use in bytes)
        pub fn sample(&mut self) -> (Option<f64>, Option<u64>) {
            // SAFETY: valid query handle.
            unsafe { PdhCollectQueryData(self.query) };
            (
                self.sum(self.util).map(|v| v.clamp(0.0, 100.0)),
                self.sum(self.mem).map(|v| v as u64),
            )
        }
    }

    impl Drop for Gpu {
        fn drop(&mut self) {
            // SAFETY: handle opened in new().
            unsafe { PdhCloseQuery(self.query) };
        }
    }

    /// (charge %, on AC power), or None without a battery.
    pub fn battery() -> Option<(u8, bool)> {
        use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
        // SAFETY: plain struct out-parameter.
        let mut s: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
        if unsafe { GetSystemPowerStatus(&mut s) } == 0
            || s.BatteryFlag & 128 != 0
            || s.BatteryLifePercent > 100
        {
            return None;
        }
        Some((s.BatteryLifePercent, s.ACLineStatus == 1))
    }

    /// Keep the display (and the PC) awake while `on`; back to normal when off.
    pub fn keep_awake(on: bool) {
        use windows_sys::Win32::System::Power::{
            ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED, SetThreadExecutionState,
        };
        let flags = if on {
            ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED
        } else {
            ES_CONTINUOUS
        };
        // SAFETY: no pointers involved.
        unsafe { SetThreadExecutionState(flags) };
    }
}

#[cfg(not(windows))]
mod imp {
    pub struct Gpu;
    impl Gpu {
        pub fn new() -> Option<Gpu> {
            None
        }
        pub fn sample(&mut self) -> (Option<f64>, Option<u64>) {
            (None, None)
        }
    }
    pub fn battery() -> Option<(u8, bool)> {
        None
    }
    pub fn keep_awake(_on: bool) {}
}

pub use imp::*;
