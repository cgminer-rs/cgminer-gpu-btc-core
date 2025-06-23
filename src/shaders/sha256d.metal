//
// SHA256d Metal 计算着色器
// 专为 Mac M4 GPU 优化的比特币挖矿着色器
//

#include <metal_stdlib>
using namespace metal;

// SHA256 常量
constant uint K[64] = {
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2
};

// SHA256 初始哈希值
constant uint H0[8] = {
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
    0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19
};

// 输入数据结构
struct MiningInput {
    uint8_t block_header[80];  // 区块头
    uint8_t target[32];        // 目标难度
    uint32_t nonce_start;      // 起始 nonce
    uint32_t nonce_count;      // nonce 数量
};

// 右旋转函数
inline uint rotr(uint x, uint n) {
    return (x >> n) | (x << (32 - n));
}

// SHA256 辅助函数
inline uint ch(uint x, uint y, uint z) {
    return (x & y) ^ (~x & z);
}

inline uint maj(uint x, uint y, uint z) {
    return (x & y) ^ (x & z) ^ (y & z);
}

inline uint sigma0(uint x) {
    return rotr(x, 2) ^ rotr(x, 13) ^ rotr(x, 22);
}

inline uint sigma1(uint x) {
    return rotr(x, 6) ^ rotr(x, 11) ^ rotr(x, 25);
}

inline uint gamma0(uint x) {
    return rotr(x, 7) ^ rotr(x, 18) ^ (x >> 3);
}

inline uint gamma1(uint x) {
    return rotr(x, 17) ^ rotr(x, 19) ^ (x >> 10);
}

// 字节序转换 (小端序到大端序)
inline uint swap_endian(uint x) {
    return ((x & 0xff) << 24) | (((x >> 8) & 0xff) << 16) |
           (((x >> 16) & 0xff) << 8) | ((x >> 24) & 0xff);
}

// SHA256 压缩函数
void sha256_compress(thread uint* state, const thread uint* block) {
    uint w[64];
    uint a, b, c, d, e, f, g, h;
    uint t1, t2;

    // 准备消息调度
    for (int i = 0; i < 16; i++) {
        w[i] = swap_endian(block[i]);
    }

    for (int i = 16; i < 64; i++) {
        w[i] = gamma1(w[i-2]) + w[i-7] + gamma0(w[i-15]) + w[i-16];
    }

    // 初始化工作变量
    a = state[0]; b = state[1]; c = state[2]; d = state[3];
    e = state[4]; f = state[5]; g = state[6]; h = state[7];

    // 主循环
    for (int i = 0; i < 64; i++) {
        t1 = h + sigma1(e) + ch(e, f, g) + K[i] + w[i];
        t2 = sigma0(a) + maj(a, b, c);
        h = g; g = f; f = e; e = d + t1;
        d = c; c = b; b = a; a = t1 + t2;
    }

    // 更新状态
    state[0] += a; state[1] += b; state[2] += c; state[3] += d;
    state[4] += e; state[5] += f; state[6] += g; state[7] += h;
}

// SHA256 哈希计算
void sha256_hash(const thread uint8_t* input, uint input_len, thread uint* output) {
    uint state[8];
    uint block[16];

    // 初始化状态
    for (int i = 0; i < 8; i++) {
        state[i] = H0[i];
    }

    // 处理完整的 512 位块
    uint blocks = input_len / 64;
    for (uint b = 0; b < blocks; b++) {
        for (int i = 0; i < 16; i++) {
            block[i] = *((thread uint*)(input + b * 64 + i * 4));
        }
        sha256_compress(state, block);
    }

    // 处理最后一块 (包含填充)
    uint remaining = input_len % 64;
    for (int i = 0; i < 16; i++) {
        block[i] = 0;
    }

    // 复制剩余字节
    for (uint i = 0; i < remaining; i++) {
        ((thread uint8_t*)block)[i] = input[blocks * 64 + i];
    }

    // 添加填充
    ((thread uint8_t*)block)[remaining] = 0x80;

    // 如果没有足够空间存放长度，需要额外一块
    if (remaining >= 56) {
        sha256_compress(state, block);
        for (int i = 0; i < 16; i++) {
            block[i] = 0;
        }
    }

    // 添加长度 (位数)
    uint64_t bit_len = input_len * 8;
    block[14] = swap_endian((uint)(bit_len >> 32));
    block[15] = swap_endian((uint)(bit_len & 0xffffffff));

    sha256_compress(state, block);

    // 输出结果
    for (int i = 0; i < 8; i++) {
        output[i] = swap_endian(state[i]);
    }
}

// 检查哈希是否满足目标难度 - thread 地址空间版本
bool check_target_thread(const thread uint* hash, const thread uint* target) {
    for (int i = 7; i >= 0; i--) {
        if (hash[i] > target[i]) return false;
        if (hash[i] < target[i]) return true;
    }
    return true;
}

// 检查哈希是否满足目标难度 - threadgroup 地址空间版本
bool check_target_threadgroup(const thread uint* hash, const threadgroup uint* target) {
    for (int i = 7; i >= 0; i--) {
        if (hash[i] > target[i]) return false;
        if (hash[i] < target[i]) return true;
    }
    return true;
}

// 主挖矿内核
kernel void sha256d_mining(
    constant MiningInput& input [[buffer(0)]],
    device uint* results [[buffer(1)]],
    uint gid [[thread_position_in_grid]]
) {
    // 计算当前线程的 nonce
    uint nonce = input.nonce_start + gid;

    // 检查边界
    if (gid >= input.nonce_count) {
        return;
    }

    // 准备区块头数据
    uint8_t header[80];
    for (int i = 0; i < 80; i++) {
        header[i] = input.block_header[i];
    }

    // 设置 nonce (在偏移量 76-79)
    *((thread uint*)(header + 76)) = nonce;

    // 第一次 SHA256
    uint first_hash[8];
    sha256_hash(header, 80, first_hash);

    // 第二次 SHA256
    uint final_hash[8];
    sha256_hash((thread uint8_t*)first_hash, 32, final_hash);

    // 检查是否满足目标难度
    uint target[8];
    for (int i = 0; i < 8; i++) {
        target[i] = *((constant uint*)(input.target + i * 4));
    }

    // 存储结果 (1 表示找到有效解，0 表示无效)
    results[gid] = check_target_thread(final_hash, target) ? 1 : 0;
}

// 性能优化版本 - 使用 threadgroup 共享内存
kernel void sha256d_mining_optimized(
    constant MiningInput& input [[buffer(0)]],
    device uint* results [[buffer(1)]],
    uint gid [[thread_position_in_grid]],
    uint lid [[thread_position_in_threadgroup]],
    uint group_size [[threads_per_threadgroup]]
) {
    // 使用 threadgroup 内存缓存常量
    threadgroup uint shared_target[8];
    threadgroup uint shared_k[64];

    // 第一个线程加载共享数据
    if (lid == 0) {
        for (int i = 0; i < 8; i++) {
            shared_target[i] = *((constant uint*)(input.target + i * 4));
        }
        for (int i = 0; i < 64; i++) {
            shared_k[i] = K[i];
        }
    }

    // 同步 threadgroup
    threadgroup_barrier(mem_flags::mem_threadgroup);

    // 计算当前线程的 nonce
    uint nonce = input.nonce_start + gid;

    // 检查边界
    if (gid >= input.nonce_count) {
        return;
    }

    // 准备区块头数据
    uint8_t header[80];
    for (int i = 0; i < 80; i++) {
        header[i] = input.block_header[i];
    }

    // 设置 nonce
    *((thread uint*)(header + 76)) = nonce;

    // 执行双重 SHA256
    uint first_hash[8];
    sha256_hash(header, 80, first_hash);

    uint final_hash[8];
    sha256_hash((thread uint8_t*)first_hash, 32, final_hash);

    // 检查目标难度
    results[gid] = check_target_threadgroup(final_hash, shared_target) ? 1 : 0;
}
