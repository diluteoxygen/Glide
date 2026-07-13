#![cfg(target_os = "windows")]

pub mod audio;
pub use audio::WasapiCapturer;

use capture_core::{CaptureError, Frame, PixelFormat, VideoCapturer};
use crossbeam_channel::Sender;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::time::Instant;
use windows::core::Interface;
use windows::Win32::Graphics::Direct3D::{
    D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL, D3D_FEATURE_LEVEL_11_0,
};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D, D3D11_CPU_ACCESS_READ,
    D3D11_CREATE_DEVICE_FLAG, D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ, D3D11_SDK_VERSION,
    D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};
use windows::Win32::Graphics::Dxgi::{
    IDXGIOutput1, IDXGIOutputDuplication, IDXGIResource, DXGI_ERROR_ACCESS_LOST,
    DXGI_ERROR_WAIT_TIMEOUT, DXGI_OUTDUPL_FRAME_INFO,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorInfo, GetIconInfo, DrawIconEx, CURSORINFO, ICONINFO, DI_NORMAL, CURSOR_SHOWING,
};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, SelectObject, DeleteObject, DeleteDC, GetObjectW,
    BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, BI_RGB,
};
use std::ffi::c_void;

pub struct DxgiCapturer {
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    duplication: IDXGIOutputDuplication,
    staging_tex: Option<ID3D11Texture2D>,
    width: u32,
    height: u32,
}

impl DxgiCapturer {
    pub fn new() -> Result<Self, CaptureError> {
        unsafe {
            // Initialize D3D11 Device
            let mut device_opt: Option<ID3D11Device> = None;
            let mut context_opt: Option<ID3D11DeviceContext> = None;
            let mut feature_level: D3D_FEATURE_LEVEL = D3D_FEATURE_LEVEL_11_0;

            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                None,
                D3D11_CREATE_DEVICE_FLAG(0),
                Some(&[D3D_FEATURE_LEVEL_11_0]),
                D3D11_SDK_VERSION,
                Some(&mut device_opt),
                Some(&mut feature_level),
                Some(&mut context_opt),
            )
            .map_err(|e| {
                CaptureError::Initialization(format!("D3D11CreateDevice failed: {}", e))
            })?;

            let device = device_opt.unwrap();
            let context = context_opt.unwrap();

            // Get DXGI Device -> Adapter -> Factory -> Output -> Duplication
            let dxgi_device: windows::Win32::Graphics::Dxgi::IDXGIDevice =
                device.cast().map_err(|e| {
                    CaptureError::Initialization(format!("cast to IDXGIDevice failed: {}", e))
                })?;
            let adapter = dxgi_device
                .GetAdapter()
                .map_err(|e| CaptureError::Initialization(format!("GetAdapter failed: {}", e)))?;

            let output = adapter
                .EnumOutputs(0)
                .map_err(|e| CaptureError::Initialization(format!("EnumOutputs failed: {}", e)))?;
            let output1: IDXGIOutput1 = output.cast().map_err(|e| {
                CaptureError::Initialization(format!("cast to IDXGIOutput1 failed: {}", e))
            })?;

            let desc = output
                .GetDesc()
                .map_err(|e| CaptureError::Initialization(format!("GetDesc failed: {}", e)))?;
            let width = (desc.DesktopCoordinates.right - desc.DesktopCoordinates.left) as u32;
            let height = (desc.DesktopCoordinates.bottom - desc.DesktopCoordinates.top) as u32;

            let duplication = output1.DuplicateOutput(&device).map_err(|e| {
                CaptureError::Initialization(format!("DuplicateOutput failed: {}", e))
            })?;

            Ok(Self {
                device,
                context,
                duplication,
                staging_tex: None,
                width,
                height,
            })
        }
    }

    fn ensure_staging_texture(&mut self) -> Result<ID3D11Texture2D, CaptureError> {
        if let Some(tex) = &self.staging_tex {
            return Ok(tex.clone());
        }

        let desc = D3D11_TEXTURE2D_DESC {
            Width: self.width,
            Height: self.height,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_STAGING,
            BindFlags: 0,
            CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
            MiscFlags: 0,
        };

        unsafe {
            let mut tex_opt = None;
            self.device
                .CreateTexture2D(&desc, None, Some(&mut tex_opt))
                .map_err(|e| CaptureError::StreamError(format!("CreateTexture2D failed: {}", e)))?;
            let tex = tex_opt.unwrap();
            self.staging_tex = Some(tex.clone());
            Ok(tex)
        }
    }
}

impl VideoCapturer for DxgiCapturer {
    fn start(
        &mut self,
        tx: Sender<Frame>,
        stop: Arc<AtomicBool>,
        dropped_frames: Arc<AtomicU64>,
        start_time: Arc<AtomicU64>,
    ) -> Result<(), CaptureError> {
        let mut qpf = 0i64;
        unsafe {
            windows::Win32::System::Performance::QueryPerformanceFrequency(&mut qpf).map_err(|e| CaptureError::Initialization(format!("QPF failed: {}", e)))?;
        }
        let qpf = qpf as u64;

        let mut frame_count = 0;
        let mut total_acquire = std::time::Duration::ZERO;
        let mut total_copy = std::time::Duration::ZERO;
        let mut total_map = std::time::Duration::ZERO;

        while !stop.load(Ordering::Relaxed) {
            let mut frame_info = DXGI_OUTDUPL_FRAME_INFO::default();
            let mut resource: Option<IDXGIResource> = None;

            let t0 = Instant::now();
            let res = unsafe {
                self.duplication.AcquireNextFrame(
                    100, // 100ms timeout
                    &mut frame_info,
                    &mut resource,
                )
            };
            total_acquire += t0.elapsed();

            match res {
                Ok(_) => {
                    // If LastPresentTime is 0, the desktop image has not been updated (e.g., just a mouse shape change).
                    if frame_info.LastPresentTime == 0 {
                        let _ = unsafe { self.duplication.ReleaseFrame() };
                        continue;
                    }

                    if let Some(res) = resource {
                        unsafe {
                            let tex: ID3D11Texture2D = res.cast().map_err(|e| {
                                CaptureError::StreamError(format!(
                                    "Cast to ID3D11Texture2D failed: {}",
                                    e
                                ))
                            })?;

                            let t1 = Instant::now();
                            let staging = self.ensure_staging_texture()?;
                            self.context.CopyResource(&staging, &tex);
                            total_copy += t1.elapsed();

                            let t2 = Instant::now();
                            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
                            self.context
                                .Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
                                .map_err(|e| {
                                    CaptureError::StreamError(format!("Map failed: {}", e))
                                })?;

                            let len = (self.width * self.height * 4) as usize;
                            let mut data = vec![0u8; len];
                            let src = mapped.pData as *const u8;
                            let pitch = mapped.RowPitch as usize;
                            let copy_width = (self.width * 4) as usize;

                            if pitch == copy_width {
                                std::ptr::copy_nonoverlapping(src, data.as_mut_ptr(), len);
                            } else {
                                let mut dst = data.as_mut_ptr();
                                for y in 0..self.height {
                                    std::ptr::copy_nonoverlapping(
                                        src.add(y as usize * pitch),
                                        dst,
                                        copy_width,
                                    );
                                    dst = dst.add(copy_width);
                                }
                            }

                            self.context.Unmap(&staging, 0);
                            let _ = self.duplication.ReleaseFrame();
                            total_map += t2.elapsed();

                            // Cursor compositing using Win32 GDI
                            let mut ci = CURSORINFO {
                                    cbSize: std::mem::size_of::<CURSORINFO>() as u32,
                                    ..Default::default()
                                };
                                if GetCursorInfo(&mut ci).is_ok() && ci.flags == CURSOR_SHOWING {
                                    let mut ii = ICONINFO::default();
                                    if GetIconInfo(ci.hCursor, &mut ii).is_ok() {
                                        let mut bm = BITMAP::default();
                                        if GetObjectW(ii.hbmMask, std::mem::size_of::<BITMAP>() as i32, Some(&mut bm as *mut _ as *mut c_void)) != 0 {
                                            let width = bm.bmWidth;
                                            let height = if ii.hbmColor.is_invalid() { bm.bmHeight / 2 } else { bm.bmHeight };

                                            let draw_x = ci.ptScreenPos.x - ii.xHotspot as i32;
                                            let draw_y = ci.ptScreenPos.y - ii.yHotspot as i32;

                                            let s_left = 0;
                                            let s_top = 0;
                                            let s_right = self.width as i32;
                                            let s_bottom = self.height as i32;

                                            let i_left = draw_x.max(s_left);
                                            let i_top = draw_y.max(s_top);
                                            let i_right = (draw_x + width).min(s_right);
                                            let i_bottom = (draw_y + height).min(s_bottom);

                                            if i_left < i_right && i_top < i_bottom {
                                                let patch_w = i_right - i_left;
                                                let patch_h = i_bottom - i_top;
                                                let dib_x = i_left - draw_x;
                                                let dib_y = i_top - draw_y;

                                                let hdc = CreateCompatibleDC(None);
                                                let bmi = BITMAPINFO {
                                                    bmiHeader: BITMAPINFOHEADER {
                                                        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                                                        biWidth: width,
                                                        biHeight: -(height as i32), // Top-down
                                                        biPlanes: 1,
                                                        biBitCount: 32,
                                                        biCompression: BI_RGB.0 as u32,
                                                        ..Default::default()
                                                    },
                                                    ..Default::default()
                                                };
                                                let mut p_bits: *mut c_void = std::ptr::null_mut();
                                                let hbm = CreateDIBSection(hdc, &bmi as *const _ as *const BITMAPINFO, DIB_RGB_COLORS, &mut p_bits, None, 0);
                                                
                                                if let Ok(hbm) = hbm {
                                                    let old_hbm = SelectObject(hdc, hbm);

                                                    // Copy valid patch from frame buffer to DIB
                                                    let p_bits_u8 = p_bits as *mut u8;
                                                    for y in 0..patch_h {
                                                        let screen_y = i_top + y;
                                                        let dib_y_actual = dib_y + y;
                                                        let screen_offset = ((screen_y * self.width as i32 + i_left) * 4) as usize;
                                                        let dib_offset = ((dib_y_actual * width + dib_x) * 4) as usize;
                                                        std::ptr::copy_nonoverlapping(
                                                            data[screen_offset..].as_ptr(),
                                                            p_bits_u8.add(dib_offset),
                                                            (patch_w * 4) as usize
                                                        );
                                                    }

                                                    // Draw cursor onto DIB
                                                    let _ = DrawIconEx(hdc, 0, 0, ci.hCursor, width, height, 0, None, DI_NORMAL);

                                                    // Copy patch back to frame buffer
                                                    for y in 0..patch_h {
                                                        let screen_y = i_top + y;
                                                        let dib_y_actual = dib_y + y;
                                                        let screen_offset = ((screen_y * self.width as i32 + i_left) * 4) as usize;
                                                        let dib_offset = ((dib_y_actual * width + dib_x) * 4) as usize;
                                                        std::ptr::copy_nonoverlapping(
                                                            p_bits_u8.add(dib_offset),
                                                            data[screen_offset..].as_mut_ptr(),
                                                            (patch_w * 4) as usize
                                                        );
                                                    }

                                                    SelectObject(hdc, old_hbm);
                                                    let _ = DeleteObject(hbm);
                                                }
                                                let _ = DeleteDC(hdc);
                                            }
                                        }
                                        if !ii.hbmColor.is_invalid() {
                                            let _ = DeleteObject(ii.hbmColor);
                                        }
                                        if !ii.hbmMask.is_invalid() {
                                            let _ = DeleteObject(ii.hbmMask);
                                        }
                                    }
                                }

                            let qpc_us = (frame_info.LastPresentTime as u64 * 1_000_000) / qpf;
                            let _ = start_time.fetch_min(qpc_us, Ordering::Relaxed);
                            let normalized_ts = qpc_us.saturating_sub(start_time.load(Ordering::Relaxed));

                            let frame = Frame {
                                data,
                                format: PixelFormat::Bgra,
                                width: self.width,
                                height: self.height,
                                timestamp_us: normalized_ts,
                            };

                            if tx.try_send(frame).is_err() {
                                dropped_frames.fetch_add(1, Ordering::Relaxed);
                            }

                            frame_count += 1;
                            if frame_count == 100 {
                                tracing::info!(
                                    "Profile (100 frames): Acquire: {:?}, Copy: {:?}, Map/Copy/Unmap: {:?}, Dropped Total: {}",
                                    total_acquire / 100,
                                    total_copy / 100,
                                    total_map / 100,
                                    dropped_frames.load(Ordering::Relaxed)
                                );
                                frame_count = 0;
                                total_acquire = std::time::Duration::ZERO;
                                total_copy = std::time::Duration::ZERO;
                                total_map = std::time::Duration::ZERO;
                            }
                        }
                    } else {
                        let _ = unsafe { self.duplication.ReleaseFrame() };
                    }
                }
                Err(e) => {
                    if e.code() == DXGI_ERROR_WAIT_TIMEOUT {
                        continue;
                    }
                    if e.code() == DXGI_ERROR_ACCESS_LOST {
                        return Err(CaptureError::StreamError("Access lost".to_string()));
                    }
                    // ignore other errors for now to keep running
                }
            }
        }

        Ok(())
    }
}
