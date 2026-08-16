use anyhow::{Result, anyhow, bail};
use image::{ImageBuffer, Rgba, RgbaImage};
use tracing::{debug, warn};
use windows::Win32::Foundation::{HMODULE, HWND, RECT};
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CPU_ACCESS_READ, D3D11_CPU_ACCESS_WRITE, D3D11_MAP_READ, D3D11_MAPPED_SUBRESOURCE,
    D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING, D3D11CreateDevice, ID3D11Device,
    ID3D11DeviceContext, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory, DXGI_ERROR_ACCESS_LOST, DXGI_OUTDUPL_FRAME_INFO, IDXGIAdapter, IDXGIFactory,
    IDXGIOutput, IDXGIOutput1, IDXGIOutputDuplication, IDXGIResource,
};
use windows::Win32::Graphics::Gdi::{HMONITOR, MONITOR_DEFAULTTONEAREST, MonitorFromWindow};
use windows::core::Interface;

use super::base::ScreencapBase;
use crate::windows_ops::WindowHandle;

// AcquireNextFrame 的超时参数
const ACQUIRE_TIMEOUT: u32 = 2000;

pub struct DesktopDupScreencap {
    hwnd: HWND,
    d3d_device: Option<ID3D11Device>,
    d3d_context: Option<ID3D11DeviceContext>,
    dxgi_factory: Option<IDXGIFactory>,
    dxgi_adapter: Option<IDXGIAdapter>,
    dxgi_output: Option<IDXGIOutput1>,
    dxgi_dup: Option<IDXGIOutputDuplication>,
    readable_texture: Option<ID3D11Texture2D>,
    texture_desc: D3D11_TEXTURE2D_DESC,
    current_monitor: HMONITOR,
    output_just_initialized: bool,
}

impl DesktopDupScreencap {
    pub fn new(window: WindowHandle) -> Self {
        Self {
            hwnd: window.0,
            d3d_device: None,
            d3d_context: None,
            dxgi_factory: None,
            dxgi_adapter: None,
            dxgi_output: None,
            dxgi_dup: None,
            readable_texture: None,
            texture_desc: D3D11_TEXTURE2D_DESC::default(),
            current_monitor: HMONITOR::default(),
            output_just_initialized: false,
        }
    }

    pub fn screencap(&mut self) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        // 初始化 D3D 设备和 DXGI 工厂（只需要初始化一次）
        if self.d3d_device.is_none()
            && let Err(e) = self.init()
        {
            warn!("failed to init_d3d_device: {:?}", e);
            self.uninit();
            return Err(e);
        }

        // 确保输出匹配当前窗口所在的显示器（每次截图时检查，支持窗口移动）
        if !self.ensure_output_for_monitor()? {
            bail!("failed to ensure_output_for_monitor");
        }

        // 如果输出刚初始化，前几张图片可能是空的，跳过
        if self.output_just_initialized {
            for _ in 0..3 {
                if let Ok(mat) = self.screencap_impl() {
                    // only check alpha of the last pixel
                    let br_alpha = mat.chunks_exact(4).last().map(|c| c[3]).unwrap_or(0);
                    if br_alpha == 255 {
                        self.output_just_initialized = false;
                        break;
                    }
                }
                debug!("blank image, continue");
            }
        }

        let mat = self.screencap_impl()?;

        // bgra_to_bgr: 交换 B 和 R 通道（对应 C++ 的 bgra_to_bgr）
        let mut bgra = mat;
        for chunk in bgra.chunks_exact_mut(4) {
            chunk.swap(0, 2); // B <-> R
        }

        ImageBuffer::from_raw(self.texture_desc.Width, self.texture_desc.Height, bgra)
            .ok_or_else(|| anyhow!("从原始数据创建图像失败"))
    }

    /// 获取当前输出（显示器）在虚拟桌面中的坐标
    pub(crate) fn output_desktop_coordinates(&self) -> Result<RECT> {
        let dxgi_output = self
            .dxgi_output
            .as_ref()
            .ok_or_else(|| anyhow!("dxgi_output is null"))?;

        let output_desc =
            unsafe { dxgi_output.GetDesc() }.map_err(|e| anyhow!("GetDesc failed: {:?}", e))?;

        Ok(output_desc.DesktopCoordinates)
    }

    fn init(&mut self) -> Result<()> {
        self.init_d3d_device()?;
        self.init_dxgi_factory()?;
        Ok(())
    }

    fn ensure_output_for_monitor(&mut self) -> Result<bool> {
        // 获取目标显示器
        let target_monitor = if !self.hwnd.is_invalid() {
            let m = unsafe { MonitorFromWindow(self.hwnd, MONITOR_DEFAULTTONEAREST) };
            if m.is_invalid() {
                warn!("MonitorFromWindow failed, falling back to primary monitor");
                HMONITOR::default()
            } else {
                m
            }
        } else {
            HMONITOR::default()
        };

        // 如果显示器没有改变，且输出已初始化，则不需要重新初始化
        if target_monitor == self.current_monitor && self.dxgi_dup.is_some() {
            return Ok(true);
        }

        // 显示器改变了或首次初始化，需要重新设置输出
        // 先释放旧的输出和纹理（因为分辨率可能改变了）
        self.dxgi_dup = None;
        self.readable_texture = None;
        self.dxgi_output = None;
        self.dxgi_adapter = None;

        // 尝试根据显示器查找输出，如果失败则使用主显示器
        let found_output = if !target_monitor.is_invalid() {
            self.find_output_by_monitor(target_monitor)?
        } else {
            false
        };

        if !found_output {
            self.init_primary_output()?;
            self.current_monitor = HMONITOR::default(); // 使用主显示器
        } else {
            self.current_monitor = target_monitor;
        }

        self.init_output_duplication()?;

        // 标记输出刚初始化，需要跳过前几张空白图片
        self.output_just_initialized = true;

        Ok(true)
    }

    fn init_d3d_device(&mut self) -> Result<()> {
        let mut d3d_device: Option<ID3D11Device> = None;
        let mut d3d_context: Option<ID3D11DeviceContext> = None;
        unsafe {
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                Default::default(),
                None,
                D3D11_SDK_VERSION,
                Some(&mut d3d_device),
                None,
                Some(&mut d3d_context),
            )
            .map_err(|e| anyhow!("D3D11CreateDevice failed: {:?}", e))?;
        }
        self.d3d_device = d3d_device;
        self.d3d_context = d3d_context;
        Ok(())
    }

    fn init_dxgi_factory(&mut self) -> Result<()> {
        let dxgi_factory: IDXGIFactory = unsafe {
            CreateDXGIFactory().map_err(|e| anyhow!("CreateDXGIFactory failed: {:?}", e))?
        };
        self.dxgi_factory = Some(dxgi_factory);
        Ok(())
    }

    fn find_output_by_monitor(&mut self, monitor: HMONITOR) -> Result<bool> {
        let dxgi_factory = match &self.dxgi_factory {
            Some(f) => f.clone(),
            None => return Ok(false),
        };

        // 遍历所有适配器，找到包含目标显示器的输出
        for adapter_index in 0.. {
            let adapter: IDXGIAdapter = match unsafe { dxgi_factory.EnumAdapters(adapter_index) } {
                Ok(a) => a,
                Err(_) => break, // 没有更多适配器了
            };

            // 遍历该适配器的所有输出
            for output_index in 0.. {
                let output: IDXGIOutput = match unsafe { adapter.EnumOutputs(output_index) } {
                    Ok(o) => o,
                    Err(_) => break, // 没有更多输出了
                };

                // 获取输出的描述信息
                if let Ok(output_desc) = unsafe { output.GetDesc() }
                    && output_desc.Monitor == monitor
                {
                    // 找到匹配的显示器
                    self.dxgi_adapter = Some(adapter);
                    self.dxgi_output = Some(
                        output
                            .cast::<IDXGIOutput1>()
                            .map_err(|e| anyhow!("cast IDXGIOutput1 failed: {:?}", e))?,
                    );
                    debug!(
                        "Found matching output for window monitor adapter={} output={}",
                        adapter_index, output_index
                    );
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    fn init_primary_output(&mut self) -> Result<()> {
        let dxgi_factory = self
            .dxgi_factory
            .as_ref()
            .ok_or_else(|| anyhow!("dxgi_factory is null"))?;

        let adapter: IDXGIAdapter = unsafe {
            dxgi_factory
                .EnumAdapters(0)
                .map_err(|e| anyhow!("EnumAdapters failed: {:?}", e))?
        };

        let output: IDXGIOutput = unsafe {
            adapter
                .EnumOutputs(0)
                .map_err(|e| anyhow!("EnumOutputs failed: {:?}", e))?
        };

        self.dxgi_adapter = Some(adapter);
        self.dxgi_output = Some(
            output
                .cast::<IDXGIOutput1>()
                .map_err(|e| anyhow!("cast IDXGIOutput1 failed: {:?}", e))?,
        );
        Ok(())
    }

    fn init_output_duplication(&mut self) -> Result<()> {
        let dxgi_output = self
            .dxgi_output
            .as_ref()
            .ok_or_else(|| anyhow!("dxgi_output is null"))?;
        let d3d_device = self
            .d3d_device
            .as_ref()
            .ok_or_else(|| anyhow!("d3d_device is null"))?;

        let dxgi_dup = unsafe {
            dxgi_output
                .DuplicateOutput(d3d_device)
                .map_err(|e| anyhow!("DuplicateOutput failed: {:?}", e))?
        };
        self.dxgi_dup = Some(dxgi_dup);
        Ok(())
    }

    fn init_texture(&mut self, raw_texture: &ID3D11Texture2D) -> Result<()> {
        let d3d_device = match &self.d3d_device {
            Some(d) => d,
            None => bail!("handle is null"),
        };

        unsafe { raw_texture.GetDesc(&mut self.texture_desc) }; // basic info

        self.texture_desc.BindFlags = Default::default();
        self.texture_desc.MiscFlags = Default::default();
        self.texture_desc.CPUAccessFlags =
            (D3D11_CPU_ACCESS_READ | D3D11_CPU_ACCESS_WRITE).0 as u32;
        self.texture_desc.Usage = D3D11_USAGE_STAGING;

        let mut readable_texture: Option<ID3D11Texture2D> = None;
        unsafe {
            d3d_device
                .CreateTexture2D(&self.texture_desc, None, Some(&mut readable_texture))
                .map_err(|e| anyhow!("CreateTexture2D failed: {:?}", e))?;
        }
        self.readable_texture = readable_texture;
        Ok(())
    }

    fn uninit(&mut self) {
        self.readable_texture = None;
        self.texture_desc = D3D11_TEXTURE2D_DESC::default();
        self.dxgi_dup = None;
        self.dxgi_output = None;
        self.dxgi_adapter = None;
        self.dxgi_factory = None;
        self.d3d_context = None;
        self.d3d_device = None;
    }

    fn screencap_impl(&mut self) -> Result<Vec<u8>> {
        let d3d_context = self
            .d3d_context
            .as_ref()
            .ok_or_else(|| anyhow!("handle is null: d3d_context"))?
            .clone();
        let dxgi_dup = self
            .dxgi_dup
            .as_ref()
            .ok_or_else(|| anyhow!("handle is null: dxgi_dup"))?
            .clone();

        let mut frame_info = DXGI_OUTDUPL_FRAME_INFO::default();
        let mut desktop_resource: Option<IDXGIResource> = None;
        let ret = unsafe {
            dxgi_dup.AcquireNextFrame(ACQUIRE_TIMEOUT, &mut frame_info, &mut desktop_resource)
        };
        if let Err(e) = ret {
            if e.code() == DXGI_ERROR_ACCESS_LOST {
                warn!("Desktop duplication access lost, reinitializing");
                self.dxgi_dup = None;
                self.readable_texture = None;
                self.current_monitor = HMONITOR::default();
                return Err(anyhow!("DXGI_ERROR_ACCESS_LOST"));
            }
            return Err(anyhow!("AcquireNextFrame failed: {:?}", e));
        }
        // OnScopeLeave 等价：dxgi_dup_->ReleaseFrame() + desktop_resource->Release()
        // desktop_resource 为 COM 智能指针，移出作用域时自动 Release
        let _frame_guard = FrameGuard(dxgi_dup.clone());

        let raw_texture: ID3D11Texture2D = desktop_resource
            .as_ref()
            .ok_or_else(|| anyhow!("desktop_resource is null"))?
            .cast()
            .map_err(|e| anyhow!("QueryInterface ID3D11Texture2D failed: {:?}", e))?;
        // OnScopeLeave 等价：raw_texture->Release() —— COM 智能指针自动处理

        if self.readable_texture.is_none() {
            self.init_texture(&raw_texture)?;
        }

        let readable_texture = self
            .readable_texture
            .as_ref()
            .ok_or_else(|| anyhow!("readable_texture is null"))?
            .clone();

        unsafe { d3d_context.CopyResource(&readable_texture, &raw_texture) };

        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        unsafe {
            d3d_context
                .Map(&readable_texture, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
                .map_err(|e| anyhow!("Map failed: {:?}", e))?;
        }
        // OnScopeLeave 等价：d3d_context_->Unmap(readable_texture_, 0)
        let _unmap_guard = UnmapGuard(d3d_context.clone(), readable_texture.clone());

        // 将映射的数据复制出来（Unmap 前必须复制）
        let width = self.texture_desc.Width as usize;
        let height = self.texture_desc.Height as usize;
        let row_pitch = mapped.RowPitch as usize;
        let src = mapped.pData as *const u8;
        let stride = width * 4;

        let mut mat = vec![0u8; width * height * 4];
        for row in 0..height {
            unsafe {
                std::ptr::copy_nonoverlapping(
                    src.add(row * row_pitch),
                    mat.as_mut_ptr().add(row * stride),
                    stride,
                );
            }
        }

        Ok(mat)
    }
}

// Win32 句柄（HWND 等）跨线程传递安全（访问时由调用方串行化）。
unsafe impl Send for DesktopDupScreencap {}

impl ScreencapBase for DesktopDupScreencap {
    fn new(window: WindowHandle) -> Self {
        Self::new(window)
    }
    fn screencap(&mut self) -> Result<RgbaImage> {
        self.screencap()
    }
}

impl Drop for DesktopDupScreencap {
    fn drop(&mut self) {
        self.uninit();
    }
}

// === RAII 守卫 ===

/// 对应 C++ OnScopeLeave: dxgi_dup_->ReleaseFrame()
struct FrameGuard(IDXGIOutputDuplication);
impl Drop for FrameGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = self.0.ReleaseFrame();
        }
    }
}

/// 对应 C++ OnScopeLeave: d3d_context_->Unmap(readable_texture_, 0)
struct UnmapGuard(ID3D11DeviceContext, ID3D11Texture2D);
impl Drop for UnmapGuard {
    fn drop(&mut self) {
        unsafe {
            self.0.Unmap(&self.1, 0);
        }
    }
}
