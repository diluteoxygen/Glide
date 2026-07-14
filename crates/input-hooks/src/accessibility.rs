use windows::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETSTICKYKEYS, SPI_GETFILTERKEYS};
use windows::Win32::UI::Accessibility::{STICKYKEYS, FILTERKEYS};
use std::mem;

pub fn are_accessibility_keys_enabled() -> bool {
    unsafe {
        let mut sticky = STICKYKEYS {
            cbSize: mem::size_of::<STICKYKEYS>() as u32,
            ..Default::default()
        };
        let mut filter = FILTERKEYS {
            cbSize: mem::size_of::<FILTERKEYS>() as u32,
            ..Default::default()
        };

        let mut enabled = false;

        if SystemParametersInfoW(SPI_GETSTICKYKEYS, sticky.cbSize, Some(&mut sticky as *mut _ as *mut _), Default::default()).is_ok() {
            // SKF_STICKYKEYSON is 0x1
            if (sticky.dwFlags.0 & 0x1) != 0 {
                enabled = true;
            }
        }

        if SystemParametersInfoW(SPI_GETFILTERKEYS, filter.cbSize, Some(&mut filter as *mut _ as *mut _), Default::default()).is_ok() {
            // FKF_FILTERKEYSON is 0x1
            if (filter.dwFlags & 0x1) != 0 {
                enabled = true;
            }
        }

        enabled
    }
}
