use anyhow::{Result, anyhow, bail};
use image::{ImageBuffer, Rgba, RgbaImage};
use tracing::{debug, error, info, warn};
use windows::Foundation::Metadata::ApiInformation;
use windows::Graphics::Capture::{
    Direct3D11CaptureFrame, Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession,
};
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Security::Authorization::AppCapabilityAccess::AppCapabilityAccessStatus;
use windows::Win32::Foundation::{HMODULE, HWND, POINT, RECT};
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CPU_ACCESS_READ, D3D11_CPU_ACCESS_WRITE, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
    D3D11_MAP_READ, D3D11_MAPPED_SUBRESOURCE, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC,
    D3D11_USAGE_STAGING, D3D11CreateDevice, D3D11CreateDeviceAndSwapChain, ID3D11Device,
    ID3D11DeviceContext, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_MODE_DESC, DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::{
    DXGI_SWAP_CHAIN_DESC, DXGI_USAGE_RENDER_TARGET_OUTPUT, IDXGIDevice, IDXGISwapChain,
};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::System::WinRT::Direct3D11::{
    CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess,
};
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::Win32::UI::WindowsAndMessaging::{
    GetClientRect, GetWindowRect, IsWindow, IsWindowVisible,
};
use windows::core::Interface;

use crate::windows_ops::WindowHandle;
use crate::windows_ops::capture::ScreencapBase;

/// FramePool 截图器
struct FramePoolScreencap {
    hwnd: HWND,
    d3d_device: Option<ID3D11Device>,
    d3d_context: Option<ID3D11DeviceContext>,
    dxgi_swap_chain: Option<IDXGISwapChain>,
    readable_texture: Option<ID3D11Texture2D>,
    texture_desc: D3D11_TEXTURE2D_DESC,

    cap_item: Option<GraphicsCaptureItem>,
    cap_frame_pool: Option<Direct3D11CaptureFramePool>,
    cap_session: Option<GraphicsCaptureSession>,

    // 存储上次的窗口大小，用于检测窗口大小变化
    last_capture_size: (i32, i32),
}
// Win32 句柄（HWND 等）跨线程传递安全（访问时由调用方串行化）。
unsafe impl Send for FramePoolScreencap {}

impl ScreencapBase for FramePoolScreencap {
    fn screencap(&mut self) -> Result<RgbaImage> {
        self.screencap()
    }
}

impl FramePoolScreencap {
    pub fn new(window: WindowHandle) -> Self {
        Self {
            hwnd: window.0,
            d3d_device: None,
            d3d_context: None,
            dxgi_swap_chain: None,
            readable_texture: None,
            texture_desc: D3D11_TEXTURE2D_DESC::default(),

            cap_item: None,
            cap_frame_pool: None,
            cap_session: None,
            last_capture_size: (0, 0),
        }
    }

    pub fn screencap(&mut self) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        if self.hwnd.is_invalid() {
            bail!("hwnd_ is nullptr");
        }
        if self.cap_frame_pool.is_none() && !self.init()? {
            error!("init failed");
            self.uninit();
            bail!("init failed");
        }

        // 检查窗口大小是否变化，如果变化则重新创建 frame pool
        if !self.check_and_handle_size_changed()? {
            bail!("check_and_handle_size_changed failed");
        }

        let frame_pool: &Direct3D11CaptureFramePool = self.cap_frame_pool.as_ref().unwrap();

        // 先清空 FramePool 中可能残留的旧帧
        while let Ok(old_frame) = frame_pool.TryGetNextFrame() {
            old_frame.Close()?;
        }

        // 等待新帧到来
        let start_time: std::time::Instant = std::time::Instant::now();
        let frame: Option<Direct3D11CaptureFrame> = loop {
            if start_time.elapsed().as_millis() >= 2000 {
                break None;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
            if let Ok(f) = frame_pool.TryGetNextFrame() {
                break Some(f);
            }
        };

        let frame: Direct3D11CaptureFrame =
            frame.ok_or_else(|| anyhow!("Failed to get frame after timeout"))?;

        let access: IDirect3DDxgiInterfaceAccess = frame.Surface()?.cast()?;

        let texture: ID3D11Texture2D = unsafe { access.GetInterface()? };

        if self.readable_texture.is_none() && !self.init_texture(&texture)? {
            bail!("falied to init_texture");
        }

        let d3d_context: &ID3D11DeviceContext = self.d3d_context.as_ref().unwrap();
        let readable_texture: &ID3D11Texture2D = self.readable_texture.as_ref().unwrap();

        unsafe { d3d_context.CopyResource(readable_texture, &texture) };

        let mut mapped: D3D11_MAPPED_SUBRESOURCE = D3D11_MAPPED_SUBRESOURCE::default();
        unsafe {
            d3d_context
                .Map(readable_texture, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
                .map_err(|e| anyhow!("Map failed: {:?}", e))?;
        }

        let _unmap_guard = UnmapGuard(d3d_context.clone(), readable_texture.clone());

        let width: u32 = self.texture_desc.Width;
        let height: u32 = self.texture_desc.Height;
        let row_pitch: usize = mapped.RowPitch as usize;
        let data_ptr: *const u8 = mapped.pData as *const u8;

        // 先按 alpha 通道裁剪掉四周 alpha != 255 的边框
        let (alpha_x, alpha_y, alpha_w, alpha_h) =
            find_alpha_bounding_rect(data_ptr, width, height, row_pitch)
                .ok_or_else(|| anyhow!("No opaque pixels found"))?;
        // let (alpha_x, alpha_y, alpha_w, alpha_h) = (0, 0, width as i32, height as i32); // 暂时不裁剪，直接使用全图

        // 获取窗口客户区矩形（相对于窗口）
        let mut client_rect = RECT::default();
        unsafe {
            GetClientRect(self.hwnd, &mut client_rect)
                .map_err(|e| anyhow!("GetClientRect failed: {:?}", e))?;
        }

        // 将客户区左上角转换为屏幕坐标
        let mut client_top_left = POINT {
            x: client_rect.left,
            y: client_rect.top,
        };
        unsafe {
            if !ClientToScreen(self.hwnd, &mut client_top_left).as_bool() {
                bail!("ClientToScreen failed");
            }
        }

        // 获取窗口矩形（屏幕坐标）
        let mut window_rect = RECT::default();
        unsafe { GetWindowRect(self.hwnd, &mut window_rect)? }

        // 计算边框位置，减去 alpha 裁剪的偏移
        let mut border_left = client_top_left.x - window_rect.left - alpha_x;
        let mut border_top = client_top_left.y - window_rect.top - alpha_y;

        // 获取客户区大小
        let mut client_width = client_rect.right - client_rect.left;
        let mut client_height = client_rect.bottom - client_rect.top;

        if border_left < 0 {
            border_left = 0;
        }
        if border_top < 0 {
            border_top = 0;
        }
        if client_width > alpha_w {
            client_width = alpha_w;
        }
        if border_left + client_width > alpha_w {
            border_left = alpha_w - client_width;
        }
        if client_height > alpha_h {
            client_height = alpha_h;
        }
        if border_top + client_height > alpha_h {
            border_top = alpha_h - client_height;
        }

        // 裁剪出客户区（去掉边框），并转换 BGRA -> RGBA
        let mut out = vec![0u8; (client_width * client_height * 4) as usize];
        unsafe {
            for row in 0..client_height as usize {
                let src_y = alpha_y as usize + border_top as usize + row;
                let src_x = alpha_x as usize + border_left as usize;
                let src = data_ptr.add(src_y * row_pitch + src_x * 4);
                let dst = out.as_mut_ptr().add(row * client_width as usize * 4);
                std::ptr::copy_nonoverlapping(src, dst, client_width as usize * 4);
            }
        }

        // bgra_to_bgr: 交换 B 和 R 通道（对应 C++ 的 bgra_to_bgr）
        for chunk in out.chunks_exact_mut(4) {
            chunk.swap(0, 2); // B <-> R
        }

        ImageBuffer::from_raw(client_width as u32, client_height as u32, out)
            .ok_or_else(|| anyhow!("从原始数据创建图像失败"))
    }

    pub fn init(&mut self) -> Result<bool> {
        if self.hwnd.is_invalid() {
            bail!("hwnd_ is nullptr");
        }

        let swap_chain_desc: DXGI_SWAP_CHAIN_DESC = DXGI_SWAP_CHAIN_DESC {
            BufferCount: 1,
            BufferDesc: DXGI_MODE_DESC {
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                ..Default::default()
            },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            OutputWindow: self.hwnd,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                ..Default::default()
            },
            Windowed: true.into(),
            ..Default::default()
        };

        unsafe {
            D3D11CreateDeviceAndSwapChain(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&swap_chain_desc),
                Some(&mut self.dxgi_swap_chain),
                Some(&mut self.d3d_device),
                None,
                Some(&mut self.d3d_context),
            )
            .map_err(|e| anyhow!("D3D11CreateDevice failed: {:?}", e))?;
        }
        if false {
            unsafe {
                D3D11CreateDevice(
                    None,
                    D3D_DRIVER_TYPE_HARDWARE,
                    HMODULE::default(),
                    D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                    None,
                    D3D11_SDK_VERSION,
                    Some(&mut self.d3d_device),
                    None,
                    Some(&mut self.d3d_context),
                )
                .map_err(|e| anyhow!("D3D11CreateDevice failed: {:?}", e))?;
            }
        }

        // 通过 IGraphicsCaptureItemInterop 为窗口创建 GraphicsCaptureItem
        let interop_factory: IGraphicsCaptureItemInterop =
            windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
        let cap_item: GraphicsCaptureItem = unsafe { interop_factory.CreateForWindow(self.hwnd)? };
        self.cap_item = Some(cap_item);

        let cap_item = self.cap_item.as_ref().unwrap();
        let item_size = cap_item.Size()?;
        if item_size.Width <= 0 || item_size.Height <= 0 {
            bail!(
                "Invalid capture item size Width={} Height={}",
                item_size.Width,
                item_size.Height
            );
        }

        unsafe {
            if !IsWindow(Some(self.hwnd)).as_bool() || !IsWindowVisible(self.hwnd).as_bool() {
                bail!("Window is no longer valid or visible");
            }
        }

        // 从 DXGIDevice 创建 WinRT IDirect3DDevice
        let dxgi_device: IDXGIDevice = self.d3d_device.as_ref().unwrap().cast()?;
        let inspectable = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device)? };
        let direct3d_device =
            inspectable.cast::<windows::Graphics::DirectX::Direct3D11::IDirect3DDevice>()?;

        let cap_frame_pool = Direct3D11CaptureFramePool::Create(
            &direct3d_device,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            1,
            item_size,
        )?;
        self.cap_frame_pool = Some(cap_frame_pool);

        let cap_session = self
            .cap_frame_pool
            .as_ref()
            .unwrap()
            .CreateCaptureSession(cap_item)?;
        self.cap_session = Some(cap_session);

        // 尝试关闭截图时的黄色边框（Windows 11 及部分 Win10 版本支持）
        self.try_disable_border();

        self.cap_session.as_ref().unwrap().StartCapture()?;

        // 记录初始窗口大小
        if let Some(item) = &self.cap_item
            && let Ok(size) = item.Size()
        {
            self.last_capture_size = (size.Width, size.Height);
        }

        Ok(true)
    }

    fn uninit(&mut self) {
        if let Some(session) = &self.cap_session {
            let _ = session.Close();
        }
        self.cap_session = None;
        self.readable_texture = None;
        self.cap_frame_pool = None;
        self.cap_session = None;
        self.texture_desc = D3D11_TEXTURE2D_DESC::default();
        self.last_capture_size = (0, 0);
    }

    fn check_and_handle_size_changed(&mut self) -> Result<bool> {
        let cap_item = match &self.cap_item {
            Some(item) => item.clone(),
            None => return Ok(true),
        };

        let current_size = cap_item.Size()?;
        // 如果窗口大小没有变化，直接返回
        if current_size.Width == self.last_capture_size.0
            && current_size.Height == self.last_capture_size.1
        {
            return Ok(true);
        }

        info!(
            "Window size changed, recreating frame pool Width={} Height={} last=({},{})",
            current_size.Width,
            current_size.Height,
            self.last_capture_size.0,
            self.last_capture_size.1
        );

        // 完全重新初始化以适应新的窗口大小
        self.uninit();
        if !self.init()? {
            bail!("reinit failed after size change");
        }

        Ok(true)
    }

    fn init_texture(&mut self, raw_texture: &ID3D11Texture2D) -> Result<bool> {
        let d3d_device = match &self.d3d_device {
            Some(dev) => dev,
            None => bail!("handle is null"),
        };

        unsafe { raw_texture.GetDesc(&mut self.texture_desc) };

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

        Ok(true)
    }

    fn try_disable_border(&self) {
        use windows::Graphics::Capture::{GraphicsCaptureAccess, GraphicsCaptureAccessKind};

        // GraphicsCaptureAccess 和 IsBorderRequired 在 UniversalApiContract v10.0 (Windows 10 2004) 引入
        match ApiInformation::IsApiContractPresentByMajor(
            windows::core::h!("Windows.Foundation.UniversalApiContract"),
            10,
        ) {
            Ok(false) | Err(_) => {
                debug!("UniversalApiContract v10 not present, border toggle not supported");
                return;
            }
            _ => {}
        }

        match ApiInformation::IsTypePresent(windows::core::h!(
            "Windows.Graphics.Capture.GraphicsCaptureAccess"
        )) {
            Ok(false) | Err(_) => {
                debug!("GraphicsCaptureAccess not present, border toggle not supported");
                return;
            }
            _ => {}
        }

        match ApiInformation::IsPropertyPresent(
            windows::core::h!("Windows.Graphics.Capture.GraphicsCaptureSession"),
            windows::core::h!("IsBorderRequired"),
        ) {
            Ok(false) | Err(_) => {
                debug!("IsBorderRequired property not supported on this system");
                return;
            }
            _ => {}
        }

        let op = match GraphicsCaptureAccess::RequestAccessAsync(
            GraphicsCaptureAccessKind::Borderless,
        ) {
            Ok(op) => op,
            Err(e) => {
                warn!("RequestAccessAsync failed: {:?}", e);
                return;
            }
        };

        let access_result = match op.get() {
            Ok(r) => r,
            Err(e) => {
                warn!("RequestAccessAsync did not complete: {:?}", e);
                return;
            }
        };

        if access_result != AppCapabilityAccessStatus::Allowed {
            warn!("Borderless capture access not granted: {:?}", access_result);
            return;
        }

        if let Some(session) = &self.cap_session
            && let Err(e) = session.SetIsBorderRequired(false)
        {
            warn!("SetIsBorderRequired failed: {:?}", e);
            return;
        }
        info!("Capture border disabled successfully");
    }
}

impl Drop for FramePoolScreencap {
    fn drop(&mut self) {
        self.uninit();
    }
}

// === RAII 守卫（对应 C++ OnScopeLeave Unmap）===
struct UnmapGuard(ID3D11DeviceContext, ID3D11Texture2D);
impl Drop for UnmapGuard {
    fn drop(&mut self) {
        unsafe { self.0.Unmap(&self.1, 0) };
    }
}

/// 对应 C++ cv::extractChannel + cv::threshold + cv::boundingRect
/// 找到 alpha == 255 的像素的最小包围矩形，返回 (x, y, width, height)
fn find_alpha_bounding_rect(
    data: *const u8,
    width: u32,
    height: u32,
    row_pitch: usize,
) -> Option<(i32, i32, i32, i32)> {
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;

    for y in 0..height as i32 {
        let row = unsafe { data.add(y as usize * row_pitch) };
        for x in 0..width as i32 {
            let alpha = unsafe { *row.add(x as usize * 4 + 3) };
            // cv::threshold(alpha_channel, alpha_bin, UCHAR_MAX-1, UCHAR_MAX, THRESH_BINARY)
            // 即 alpha > 254，等价于 alpha == 255
            if alpha == u8::MAX {
                if x < min_x {
                    min_x = x;
                }
                if y < min_y {
                    min_y = y;
                }
                if x > max_x {
                    max_x = x;
                }
                if y > max_y {
                    max_y = y;
                }
            }
        }
    }

    if min_x > max_x {
        return None; // No opaque pixels found
    }

    Some((min_x, min_y, max_x - min_x + 1, max_y - min_y + 1))
}
