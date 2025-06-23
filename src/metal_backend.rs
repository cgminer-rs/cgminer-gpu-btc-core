//! Mac Metal GPU 后端实现
//!
//! 专门为 Mac M4 GPU 优化的 Metal 计算着色器实现，
//! 提供高性能的 SHA256d 并行计算能力。

#[cfg(feature = "mac-metal")]
use metal::*;
#[cfg(feature = "mac-metal")]
use objc::runtime::Object;
#[cfg(feature = "mac-metal")]
use std::sync::Arc;

use cgminer_core::{DeviceError, Work, MiningResult};
use tracing::{info, warn, error, debug};
use std::collections::HashMap;

/// Metal GPU 设备信息
#[derive(Debug, Clone)]
pub struct MetalDeviceInfo {
    pub device_id: u32,
    pub name: String,
    pub max_threads_per_threadgroup: u32,
    pub max_buffer_length: u64,
    pub supports_non_uniform_threadgroups: bool,
    pub recommended_max_working_set_size: u64,
}

/// Metal GPU 后端
pub struct MetalBackend {
    #[cfg(feature = "mac-metal")]
    device: Device,
    #[cfg(feature = "mac-metal")]
    command_queue: CommandQueue,
    #[cfg(feature = "mac-metal")]
    compute_pipeline: Option<ComputePipelineState>,
    #[cfg(feature = "mac-metal")]
    library: Option<Library>,

    device_info: MetalDeviceInfo,
    is_initialized: bool,
}

impl MetalBackend {
    /// 创建新的 Metal 后端
    pub fn new() -> Result<Self, DeviceError> {
        #[cfg(feature = "mac-metal")]
        {
            let device = Device::system_default()
                .ok_or_else(|| DeviceError::initialization_failed("无法获取系统默认 Metal 设备".to_string()))?;

            let device_info = Self::get_device_info_static(&device)?;
            info!("🖥️ 检测到 Metal 设备: {}", device_info.name);

            Ok(Self {
                device: device.clone(),
                command_queue: device.new_command_queue(),
                compute_pipeline: None,
                library: None,
                device_info,
                is_initialized: false,
            })
        }

        #[cfg(not(feature = "mac-metal"))]
        {
            Err(DeviceError::unsupported_operation("Metal 支持未启用".to_string()))
        }
    }

    /// 初始化 Metal 后端
    pub async fn initialize(&mut self) -> Result<(), DeviceError> {
        #[cfg(feature = "mac-metal")]
        {
            info!("🚀 初始化 Metal GPU 后端");

            // 创建命令队列
            self.command_queue = self.device.new_command_queue();

            // 编译 Metal 着色器
            self.compile_shaders().await?;

            self.is_initialized = true;
            info!("✅ Metal GPU 后端初始化完成");
            Ok(())
        }

        #[cfg(not(feature = "mac-metal"))]
        {
            Err(DeviceError::unsupported_operation("Metal 支持未启用".to_string()))
        }
    }

    /// 编译 Metal 计算着色器
    #[cfg(feature = "mac-metal")]
    async fn compile_shaders(&mut self) -> Result<(), DeviceError> {
        let shader_source = include_str!("shaders/sha256d.metal");

        let library = self.device.new_library_with_source(shader_source, &CompileOptions::new())
            .map_err(|e| DeviceError::initialization_failed(format!("编译 Metal 着色器失败: {:?}", e)))?;

        let function = library.get_function("sha256d_mining", None)
            .map_err(|_| DeviceError::initialization_failed("找不到 sha256d_mining 函数".to_string()))?;

        self.compute_pipeline = Some(self.device.new_compute_pipeline_state_with_function(&function)
            .map_err(|e| DeviceError::initialization_failed(format!("创建计算管线失败: {:?}", e)))?);

        self.library = Some(library);

        debug!("✅ Metal 着色器编译完成");
        Ok(())
    }

    /// 获取设备信息（静态方法）
    #[cfg(feature = "mac-metal")]
    fn get_device_info_static(device: &Device) -> Result<MetalDeviceInfo, DeviceError> {
        Ok(MetalDeviceInfo {
            device_id: 0, // Mac 通常只有一个 GPU
            name: device.name().to_string(),
            max_threads_per_threadgroup: device.max_threads_per_threadgroup().width as u32,
            max_buffer_length: device.max_buffer_length(),
            supports_non_uniform_threadgroups: true, // 假设支持
            recommended_max_working_set_size: device.recommended_max_working_set_size(),
        })
    }

    /// 执行挖矿计算
    pub async fn mine(&self, work: &Work, nonce_start: u32, nonce_count: u32) -> Result<Vec<MiningResult>, DeviceError> {
        #[cfg(feature = "mac-metal")]
        {
            if !self.is_initialized {
                return Err(DeviceError::hardware_error("Metal 后端未初始化".to_string()));
            }

            debug!("🔨 开始 Metal GPU 挖矿: nonce_start={}, count={}", nonce_start, nonce_count);

            // 准备输入数据
            let input_data = self.prepare_input_data(work, nonce_start, nonce_count)?;

            // 创建 Metal 缓冲区
            let input_buffer = self.device.new_buffer_with_data(
                input_data.as_ptr() as *const std::ffi::c_void,
                input_data.len() as u64,
                MTLResourceOptions::StorageModeShared,
            );

            let output_size = nonce_count as u64 * std::mem::size_of::<u32>() as u64;
            let output_buffer = self.device.new_buffer(output_size, MTLResourceOptions::StorageModeShared);

            // 创建命令缓冲区
            let command_buffer = self.command_queue.new_command_buffer();
            let compute_encoder = command_buffer.new_compute_command_encoder();

            // 设置计算管线和缓冲区
            if let Some(ref pipeline) = self.compute_pipeline {
                compute_encoder.set_compute_pipeline_state(pipeline);
            } else {
                return Err(DeviceError::hardware_error("计算管线未初始化".to_string()));
            }
            compute_encoder.set_buffer(0, Some(&input_buffer), 0);
            compute_encoder.set_buffer(1, Some(&output_buffer), 0);

            // 计算线程组大小
            let threads_per_threadgroup = MTLSize::new(
                std::cmp::min(self.device_info.max_threads_per_threadgroup as u64, nonce_count as u64),
                1,
                1,
            );

            let threadgroups = MTLSize::new(
                (nonce_count as u64 + threads_per_threadgroup.width - 1) / threads_per_threadgroup.width,
                1,
                1,
            );

            // 分发计算任务
            compute_encoder.dispatch_thread_groups(threadgroups, threads_per_threadgroup);
            compute_encoder.end_encoding();

            // 提交并等待完成
            command_buffer.commit();
            command_buffer.wait_until_completed();

            // 读取结果
            let results = self.process_results(&output_buffer, work, nonce_start, nonce_count)?;

            debug!("✅ Metal GPU 挖矿完成，找到 {} 个结果", results.len());
            Ok(results)
        }

        #[cfg(not(feature = "mac-metal"))]
        {
            Err(DeviceError::unsupported_operation("Metal 支持未启用".to_string()))
        }
    }

    /// 准备输入数据
    fn prepare_input_data(&self, work: &Work, nonce_start: u32, nonce_count: u32) -> Result<Vec<u8>, DeviceError> {
        let mut data = Vec::new();

        // 添加区块头数据 (80 字节)
        data.extend_from_slice(&work.header);

        // 添加目标难度
        data.extend_from_slice(&work.target);

        // 添加 nonce 范围
        data.extend_from_slice(&nonce_start.to_le_bytes());
        data.extend_from_slice(&nonce_count.to_le_bytes());

        Ok(data)
    }

    /// 处理挖矿结果
    #[cfg(feature = "mac-metal")]
    fn process_results(
        &self,
        output_buffer: &Buffer,
        work: &Work,
        nonce_start: u32,
        nonce_count: u32,
    ) -> Result<Vec<MiningResult>, DeviceError> {
        let mut results = Vec::new();

        // 读取输出缓冲区
        let output_ptr = output_buffer.contents() as *const u32;
        let output_slice = unsafe {
            std::slice::from_raw_parts(output_ptr, nonce_count as usize)
        };

        // 检查每个 nonce 的结果
        for (i, &result) in output_slice.iter().enumerate() {
            if result > 0 {
                // 找到有效解
                let nonce = nonce_start + i as u32;
                let mining_result = MiningResult {
                    work_id: work.id,
                    work_id_numeric: work.work_id,
                    nonce,
                    extranonce2: vec![],
                    hash: self.calculate_hash(work, nonce)?,
                    share_difficulty: work.difficulty,
                    meets_target: true,
                    timestamp: std::time::SystemTime::now(),
                    device_id: self.device_info.device_id,
                };
                results.push(mining_result);
            }
        }

        Ok(results)
    }

    /// 计算哈希值 (用于验证)
    fn calculate_hash(&self, work: &Work, nonce: u32) -> Result<Vec<u8>, DeviceError> {
        use sha2::{Sha256, Digest};

        let mut header = work.header.clone();
        // 替换 nonce (在偏移量 76-79)
        header[76..80].copy_from_slice(&nonce.to_le_bytes());

        // 双重 SHA256
        let first_hash = Sha256::digest(&header);
        let second_hash = Sha256::digest(&first_hash);

        Ok(second_hash.to_vec())
    }

    /// 获取设备信息
    pub fn get_device_info(&self) -> &MetalDeviceInfo {
        &self.device_info
    }

    /// 检查是否已初始化
    pub fn is_initialized(&self) -> bool {
        self.is_initialized
    }

    /// 获取推荐的工作组大小
    pub fn get_optimal_work_size(&self) -> u32 {
        // 基于设备能力计算最优工作组大小
        let base_size = self.device_info.max_threads_per_threadgroup;

        // Mac M4 GPU 优化：使用较大的工作组以充分利用并行性
        std::cmp::min(base_size * 32, 65536)
    }
}

impl Default for MetalBackend {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            // 如果无法创建真实的 Metal 后端，创建一个占位符
            Self {
                #[cfg(feature = "mac-metal")]
                device: unsafe { std::mem::zeroed() },
                #[cfg(feature = "mac-metal")]
                command_queue: unsafe { std::mem::zeroed() },
                #[cfg(feature = "mac-metal")]
                compute_pipeline: None,
                #[cfg(feature = "mac-metal")]
                library: None,

                device_info: MetalDeviceInfo {
                    device_id: 0,
                    name: "Mock Metal Device".to_string(),
                    max_threads_per_threadgroup: 1024,
                    max_buffer_length: 1024 * 1024 * 1024,
                    supports_non_uniform_threadgroups: true,
                    recommended_max_working_set_size: 512 * 1024 * 1024,
                },
                is_initialized: false,
            }
        })
    }
}

/// Metal 后端错误类型
#[derive(Debug, thiserror::Error)]
pub enum MetalError {
    #[error("设备初始化失败: {0}")]
    DeviceInitialization(String),

    #[error("着色器编译失败: {0}")]
    ShaderCompilation(String),

    #[error("计算执行失败: {0}")]
    ComputeExecution(String),

    #[error("不支持的操作: {0}")]
    Unsupported(String),
}

impl From<MetalError> for DeviceError {
    fn from(error: MetalError) -> Self {
        match error {
            MetalError::DeviceInitialization(msg) => DeviceError::initialization_failed(msg),
            MetalError::ShaderCompilation(msg) => DeviceError::hardware_error(msg),
            MetalError::ComputeExecution(msg) => DeviceError::hardware_error(msg),
            MetalError::Unsupported(msg) => DeviceError::unsupported_operation(msg),
        }
    }
}
