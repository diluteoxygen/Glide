#![cfg(target_os = "windows")]

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
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
    D3D11_CPU_ACCESS_READ, D3D11_CREATE_DEVICE_FLAG, D3D11_MAP_READ, D3D11_MAPPED_SUBRESOURCE,
    D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING, D3D11_SDK_VERSION,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};
use windows::Win32::Graphics::Dxgi::{
    IDXGIOutput1, IDXGIOutputDuplication,
    IDXGIResource, DXGI_ERROR_ACCESS_LOST, DXGI_ERROR_WAIT_TIMEOUT,
    DXGI_OUTDUPL_FRAME_INFO,
};

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
            ).map_err(|e| CaptureError::Initialization(format!("D3D11CreateDevice failed: {}", e)))?;

            let device = device_opt.unwrap();
            let context = context_opt.unwrap();

            // Get DXGI Device -> Adapter -> Factory -> Output -> Duplication
            let dxgi_device: windows::Win32::Graphics::Dxgi::IDXGIDevice = device.cast().map_err(|e| CaptureError::Initialization(format!("cast to IDXGIDevice failed: {}", e)))?;
            let adapter = dxgi_device.GetAdapter().map_err(|e| CaptureError::Initialization(format!("GetAdapter failed: {}", e)))?;
            
            let output = adapter.EnumOutputs(0).map_err(|e| CaptureError::Initialization(format!("EnumOutputs failed: {}", e)))?;
            let output1: IDXGIOutput1 = output.cast().map_err(|e| CaptureError::Initialization(format!("cast to IDXGIOutput1 failed: {}", e)))?;
            
            let desc = output.GetDesc().map_err(|e| CaptureError::Initialization(format!("GetDesc failed: {}", e)))?;
            let width = (desc.DesktopCoordinates.right - desc.DesktopCoordinates.left) as u32;
            let height = (desc.DesktopCoordinates.bottom - desc.DesktopCoordinates.top) as u32;

            let duplication = output1.DuplicateOutput(&device).map_err(|e| CaptureError::Initialization(format!("DuplicateOutput failed: {}", e)))?;

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
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Usage: D3D11_USAGE_STAGING,
            BindFlags: 0,
            CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
            MiscFlags: 0,
        };
        
        unsafe {
            let mut tex_opt = None;
            self.device.CreateTexture2D(&desc, None, Some(&mut tex_opt))
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
    ) -> Result<(), CaptureError> {
        let start_time = Instant::now();
        
        while !stop.load(Ordering::Relaxed) {
            let mut frame_info = DXGI_OUTDUPL_FRAME_INFO::default();
            let mut resource: Option<IDXGIResource> = None;
            
            let res = unsafe {
                self.duplication.AcquireNextFrame(
                    100, // 100ms timeout
                    &mut frame_info,
                    &mut resource,
                )
            };
            
            match res {
                Ok(_) => {
                    if let Some(res) = resource {
                        unsafe {
                            let tex: ID3D11Texture2D = res.cast()
                                .map_err(|e| CaptureError::StreamError(format!("Cast to ID3D11Texture2D failed: {}", e)))?;
                                
                            let staging = self.ensure_staging_texture()?;
                            
                            self.context.CopyResource(&staging, &tex);
                            
                            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
                            self.context.Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
                                .map_err(|e| CaptureError::StreamError(format!("Map failed: {}", e)))?;
                                
                            let mut data = Vec::with_capacity((self.width * self.height * 4) as usize);
                            let src = mapped.pData as *const u8;
                            let pitch = mapped.RowPitch as usize;
                            let copy_width = (self.width * 4) as usize;
                            
                            for y in 0..self.height {
                                let row_start = src.add(y as usize * pitch);
                                data.extend_from_slice(std::slice::from_raw_parts(row_start, copy_width));
                            }
                            
                            self.context.Unmap(&staging, 0);
                            
                            let _ = self.duplication.ReleaseFrame();
                            
                            let frame = Frame {
                                data,
                                format: PixelFormat::Bgra,
                                width: self.width,
                                height: self.height,
                                timestamp_us: start_time.elapsed().as_micros() as u64,
                            };
                            
                            if tx.try_send(frame).is_err() {
                                dropped_frames.fetch_add(1, Ordering::Relaxed);
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
