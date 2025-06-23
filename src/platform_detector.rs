//! GPU 平台检测模块
//! 
//! 自动检测当前系统的 GPU 平台，并选择最适合的后端实现。
//! 优先级：Mac Metal > NVIDIA CUDA > AMD OpenCL > Intel OpenCL

use cgminer_core::{DeviceInfo, DeviceError};
use tracing::{info, warn, debug};
use std::collections::HashMap;

/// GPU 平台类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpuPlatform {
    /// Mac Metal (Apple Silicon)
    MacMetal,
    /// NVIDIA CUDA
    NvidiaCuda,
    /// AMD OpenCL
    AmdOpenCl,
    /// Intel OpenCL
    IntelOpenCl,
    /// 通用 OpenCL
    GenericOpenCl,
    /// 未知平台
    Unknown,
}

impl GpuPlatform {
    /// 获取平台名称
    pub fn name(&self) -> &'static str {
        match self {
            GpuPlatform::MacMetal => "Mac Metal",
            GpuPlatform::NvidiaCuda => "NVIDIA CUDA",
            GpuPlatform::AmdOpenCl => "AMD OpenCL",
            GpuPlatform::IntelOpenCl => "Intel OpenCL",
            GpuPlatform::GenericOpenCl => "Generic OpenCL",
            GpuPlatform::Unknown => "Unknown",
        }
    }

    /// 获取平台优先级 (数值越小优先级越高)
    pub fn priority(&self) -> u32 {
        match self {
            GpuPlatform::MacMetal => 1,
            GpuPlatform::NvidiaCuda => 2,
            GpuPlatform::AmdOpenCl => 3,
            GpuPlatform::IntelOpenCl => 4,
            GpuPlatform::GenericOpenCl => 5,
            GpuPlatform::Unknown => 999,
        }
    }

    /// 检查平台是否可用
    pub async fn is_available(&self) -> bool {
        match self {
            GpuPlatform::MacMetal => self.check_metal_availability().await,
            GpuPlatform::NvidiaCuda => self.check_cuda_availability().await,
            GpuPlatform::AmdOpenCl => self.check_amd_opencl_availability().await,
            GpuPlatform::IntelOpenCl => self.check_intel_opencl_availability().await,
            GpuPlatform::GenericOpenCl => self.check_generic_opencl_availability().await,
            GpuPlatform::Unknown => false,
        }
    }

    /// 检查 Mac Metal 可用性
    async fn check_metal_availability(&self) -> bool {
        #[cfg(all(feature = "mac-metal", target_os = "macos"))]
        {
            use crate::metal_backend::MetalBackend;
            match MetalBackend::new() {
                Ok(_) => {
                    debug!("✅ Mac Metal 平台可用");
                    true
                }
                Err(_) => {
                    debug!("❌ Mac Metal 平台不可用");
                    false
                }
            }
        }
        
        #[cfg(not(all(feature = "mac-metal", target_os = "macos")))]
        {
            debug!("❌ Mac Metal 平台不支持 (非 macOS 或特性未启用)");
            false
        }
    }

    /// 检查 NVIDIA CUDA 可用性
    async fn check_cuda_availability(&self) -> bool {
        #[cfg(feature = "cuda")]
        {
            // TODO: 实际的 CUDA 可用性检查
            // - 检查 CUDA 驱动
            // - 检查 CUDA 运行时
            // - 检查 NVIDIA GPU
            debug!("🔍 检查 NVIDIA CUDA 可用性 (预留实现)");
            false // 暂时返回 false，等待实际实现
        }
        
        #[cfg(not(feature = "cuda"))]
        {
            debug!("❌ NVIDIA CUDA 平台不支持 (特性未启用)");
            false
        }
    }

    /// 检查 AMD OpenCL 可用性
    async fn check_amd_opencl_availability(&self) -> bool {
        #[cfg(feature = "opencl")]
        {
            // TODO: 实际的 AMD OpenCL 可用性检查
            // - 检查 OpenCL 平台
            // - 检查 AMD GPU 设备
            debug!("🔍 检查 AMD OpenCL 可用性 (预留实现)");
            false // 暂时返回 false，等待实际实现
        }
        
        #[cfg(not(feature = "opencl"))]
        {
            debug!("❌ AMD OpenCL 平台不支持 (特性未启用)");
            false
        }
    }

    /// 检查 Intel OpenCL 可用性
    async fn check_intel_opencl_availability(&self) -> bool {
        #[cfg(feature = "opencl")]
        {
            // TODO: 实际的 Intel OpenCL 可用性检查
            // - 检查 OpenCL 平台
            // - 检查 Intel GPU 设备
            debug!("🔍 检查 Intel OpenCL 可用性 (预留实现)");
            false // 暂时返回 false，等待实际实现
        }
        
        #[cfg(not(feature = "opencl"))]
        {
            debug!("❌ Intel OpenCL 平台不支持 (特性未启用)");
            false
        }
    }

    /// 检查通用 OpenCL 可用性
    async fn check_generic_opencl_availability(&self) -> bool {
        #[cfg(feature = "opencl")]
        {
            // TODO: 实际的通用 OpenCL 可用性检查
            // - 检查任何可用的 OpenCL 平台
            debug!("🔍 检查通用 OpenCL 可用性 (预留实现)");
            false // 暂时返回 false，等待实际实现
        }
        
        #[cfg(not(feature = "opencl"))]
        {
            debug!("❌ 通用 OpenCL 平台不支持 (特性未启用)");
            false
        }
    }
}

/// GPU 平台检测器
pub struct PlatformDetector {
    /// 检测到的平台
    detected_platforms: Vec<GpuPlatform>,
    /// 平台详细信息
    platform_info: HashMap<GpuPlatform, String>,
}

impl PlatformDetector {
    /// 创建新的平台检测器
    pub fn new() -> Self {
        Self {
            detected_platforms: Vec::new(),
            platform_info: HashMap::new(),
        }
    }

    /// 检测所有可用的 GPU 平台
    pub async fn detect_platforms(&mut self) -> Result<Vec<GpuPlatform>, DeviceError> {
        info!("🔍 开始检测 GPU 平台");

        let all_platforms = vec![
            GpuPlatform::MacMetal,
            GpuPlatform::NvidiaCuda,
            GpuPlatform::AmdOpenCl,
            GpuPlatform::IntelOpenCl,
            GpuPlatform::GenericOpenCl,
        ];

        let mut available_platforms = Vec::new();

        for platform in all_platforms {
            debug!("🔍 检测平台: {}", platform.name());
            
            if platform.is_available().await {
                info!("✅ 发现可用平台: {}", platform.name());
                available_platforms.push(platform.clone());
                
                // 收集平台详细信息
                let info = self.get_platform_info(&platform).await;
                self.platform_info.insert(platform, info);
            } else {
                debug!("❌ 平台不可用: {}", platform.name());
            }
        }

        // 按优先级排序
        available_platforms.sort_by_key(|p| p.priority());

        self.detected_platforms = available_platforms.clone();

        if available_platforms.is_empty() {
            warn!("⚠️ 未检测到任何可用的 GPU 平台");
        } else {
            info!("🎯 检测到 {} 个可用平台，优先使用: {}", 
                  available_platforms.len(), 
                  available_platforms[0].name());
        }

        Ok(available_platforms)
    }

    /// 获取最佳平台
    pub fn get_best_platform(&self) -> Option<&GpuPlatform> {
        self.detected_platforms.first()
    }

    /// 获取所有检测到的平台
    pub fn get_detected_platforms(&self) -> &[GpuPlatform] {
        &self.detected_platforms
    }

    /// 获取平台详细信息
    pub fn get_platform_details(&self, platform: &GpuPlatform) -> Option<&String> {
        self.platform_info.get(platform)
    }

    /// 检查特定平台是否可用
    pub fn is_platform_available(&self, platform: &GpuPlatform) -> bool {
        self.detected_platforms.contains(platform)
    }

    /// 获取平台信息
    async fn get_platform_info(&self, platform: &GpuPlatform) -> String {
        match platform {
            GpuPlatform::MacMetal => {
                #[cfg(all(feature = "mac-metal", target_os = "macos"))]
                {
                    use crate::metal_backend::MetalBackend;
                    if let Ok(backend) = MetalBackend::new() {
                        let info = backend.get_device_info();
                        return format!("设备: {}, 最大线程组: {}, 内存: {} MB", 
                                     info.name, 
                                     info.max_threads_per_threadgroup,
                                     info.max_buffer_length / 1024 / 1024);
                    }
                }
                "Mac Metal GPU".to_string()
            }
            GpuPlatform::NvidiaCuda => {
                // TODO: 获取 CUDA 设备信息
                "NVIDIA CUDA GPU (预留)".to_string()
            }
            GpuPlatform::AmdOpenCl => {
                // TODO: 获取 AMD OpenCL 设备信息
                "AMD OpenCL GPU (预留)".to_string()
            }
            GpuPlatform::IntelOpenCl => {
                // TODO: 获取 Intel OpenCL 设备信息
                "Intel OpenCL GPU (预留)".to_string()
            }
            GpuPlatform::GenericOpenCl => {
                // TODO: 获取通用 OpenCL 设备信息
                "Generic OpenCL GPU (预留)".to_string()
            }
            GpuPlatform::Unknown => "Unknown GPU".to_string(),
        }
    }

    /// 根据平台创建设备信息
    pub async fn create_device_info_for_platform(
        &self, 
        platform: &GpuPlatform, 
        device_id: u32
    ) -> Result<DeviceInfo, DeviceError> {
        let device_name = match platform {
            GpuPlatform::MacMetal => {
                #[cfg(all(feature = "mac-metal", target_os = "macos"))]
                {
                    use crate::metal_backend::MetalBackend;
                    if let Ok(backend) = MetalBackend::new() {
                        format!("Mac Metal GPU: {}", backend.get_device_info().name)
                    } else {
                        "Mac Metal GPU".to_string()
                    }
                }
                #[cfg(not(all(feature = "mac-metal", target_os = "macos")))]
                "Mac Metal GPU".to_string()
            }
            GpuPlatform::NvidiaCuda => format!("NVIDIA CUDA GPU {}", device_id),
            GpuPlatform::AmdOpenCl => format!("AMD OpenCL GPU {}", device_id),
            GpuPlatform::IntelOpenCl => format!("Intel OpenCL GPU {}", device_id),
            GpuPlatform::GenericOpenCl => format!("Generic OpenCL GPU {}", device_id),
            GpuPlatform::Unknown => format!("Unknown GPU {}", device_id),
        };

        let device_type = match platform {
            GpuPlatform::MacMetal => "mac-metal",
            GpuPlatform::NvidiaCuda => "nvidia-cuda",
            GpuPlatform::AmdOpenCl => "amd-opencl",
            GpuPlatform::IntelOpenCl => "intel-opencl",
            GpuPlatform::GenericOpenCl => "generic-opencl",
            GpuPlatform::Unknown => "unknown",
        };

        Ok(DeviceInfo::new(
            device_id,
            device_name,
            device_type.to_string(),
            0, // GPU 通常没有链的概念
        ))
    }

    /// 获取推荐的设备数量
    pub fn get_recommended_device_count(&self, platform: &GpuPlatform) -> u32 {
        match platform {
            GpuPlatform::MacMetal => 1, // Mac 通常只有一个 GPU
            GpuPlatform::NvidiaCuda => 1, // 默认使用一个 CUDA 设备
            GpuPlatform::AmdOpenCl => 1, // 默认使用一个 AMD 设备
            GpuPlatform::IntelOpenCl => 1, // 默认使用一个 Intel 设备
            GpuPlatform::GenericOpenCl => 1, // 默认使用一个 OpenCL 设备
            GpuPlatform::Unknown => 0,
        }
    }
}

impl Default for PlatformDetector {
    fn default() -> Self {
        Self::new()
    }
}
