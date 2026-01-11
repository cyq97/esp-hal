# Design Document: UDP Communication

## Overview

本设计文档描述了基于ESP32系列芯片的UDP通信工程实现。该工程基于 `demo_static_ip` 示例，使用 `esp-radio` 和 `smoltcp` 网络协议栈实现UDP客户端和服务器功能。系统将使用静态IP配置，支持双向UDP通信。

## Architecture

系统采用分层架构：

```
┌─────────────────────────────────────┐
│     Application Layer               │
│  (UDP Client/Server Logic)          │
├─────────────────────────────────────┤
│     Network Stack Layer             │
│  (blocking-network-stack + smoltcp) │
├─────────────────────────────────────┤
│     WiFi Driver Layer               │
│  (esp-radio WiFi)                   │
├─────────────────────────────────────┤
│     Hardware Abstraction Layer      │
│  (esp-hal)                          │
└─────────────────────────────────────┘
```

### 主要模块

1. **WiFi初始化模块**: 负责WiFi连接和静态IP配置
2. **UDP Socket管理模块**: 管理UDP socket的创建、绑定和数据传输
3. **数据处理模块**: 处理UDP数据包的发送和接收
4. **配置管理模块**: 从环境变量读取配置参数

## Components and Interfaces

### 1. WiFi Manager Component

**职责**: 初始化WiFi连接并配置静态IP

**接口**:
```rust
fn initialize_wifi(
    peripherals: Peripherals,
    ssid: &str,
    password: &str,
) -> (WifiController, WifiDevice)

fn configure_static_ip(
    stack: &mut Stack,
    ip: [u8; 4],
    gateway: [u8; 4],
    netmask: u8,
) -> Result<(), NetworkError>
```

### 2. UDP Socket Component

**职责**: 管理UDP socket的生命周期和数据传输

**接口**:
```rust
// UDP服务器模式
fn bind_udp_server(
    socket: &mut UdpSocket,
    port: u16,
) -> Result<(), SocketError>

fn receive_udp(
    socket: &mut UdpSocket,
    buffer: &mut [u8],
) -> Result<(usize, SocketAddr), SocketError>

fn send_udp_response(
    socket: &mut UdpSocket,
    data: &[u8],
    addr: SocketAddr,
) -> Result<usize, SocketError>

// UDP客户端模式
fn send_udp_to(
    socket: &mut UdpSocket,
    data: &[u8],
    target_ip: [u8; 4],
    target_port: u16,
) -> Result<usize, SocketError>
```

### 3. Configuration Component

**职责**: 解析和验证环境变量配置

**接口**:
```rust
struct UdpConfig {
    ssid: &'static str,
    password: &'static str,
    static_ip: [u8; 4],
    gateway_ip: [u8; 4],
    udp_port: u16,
    target_ip: Option<[u8; 4]>,
    target_port: Option<u16>,
}

fn load_config() -> UdpConfig
fn parse_ip(ip_str: &str) -> [u8; 4]
```

## Data Models

### UDP Packet Structure

```rust
struct UdpPacket {
    data: [u8; MAX_UDP_PAYLOAD],
    length: usize,
    source_addr: SocketAddr,
}

const MAX_UDP_PAYLOAD: usize = 1472; // 以太网MTU (1500) - IP头(20) - UDP头(8)
```

### Socket Address

```rust
struct SocketAddr {
    ip: [u8; 4],
    port: u16,
}
```

### Network Configuration

```rust
struct NetworkConfig {
    ip_address: [u8; 4],
    gateway: [u8; 4],
    netmask: u8,
}
```

## Correctness Properties

*属性（Property）是系统在所有有效执行中应该保持为真的特征或行为——本质上是关于系统应该做什么的形式化陈述。属性是人类可读规范和机器可验证正确性保证之间的桥梁。*


### Property 1: 静态IP配置正确性
*对于任何*有效的IP地址和网关配置，配置完成后查询网络接口应该返回相同的IP地址和网关
**Validates: Requirements 1.2, 1.3**

### Property 2: UDP服务器端口绑定
*对于任何*有效的端口号（1024-65535），UDP服务器应该能够成功绑定到该端口
**Validates: Requirements 2.1**

### Property 3: UDP数据包接收和回复
*对于任何*接收到的有效UDP数据包，服务器应该能够解析数据包内容并向发送方发送回复
**Validates: Requirements 2.2, 2.3**

### Property 4: 连续数据包处理
*对于任何*数量的连续UDP数据包（N个，N > 0），服务器应该能够依次处理所有数据包
**Validates: Requirements 2.4**

### Property 5: UDP客户端配置
*对于任何*有效的目标IP地址和端口号，UDP客户端应该能够正确配置目标服务器信息
**Validates: Requirements 3.1**

### Property 6: UDP数据往返一致性
*对于任何*有效的数据（文本或二进制），发送到服务器后接收到的回复应该包含原始数据的确认信息
**Validates: Requirements 3.2, 3.3, 4.1, 4.2**

### Property 7: 数据包完整性验证
*对于任何*接收到的UDP数据包，系统应该验证数据包长度与实际接收的字节数一致
**Validates: Requirements 4.3**

### Property 8: 数据包信息日志记录
*对于任何*发送或接收的UDP数据包，系统日志应该包含时间戳、数据包大小、源地址和目标地址信息
**Validates: Requirements 5.3**

## Error Handling

### WiFi连接错误
- **连接超时**: 如果WiFi连接在30秒内未成功，记录错误并重试（最多3次）
- **认证失败**: 记录详细的认证错误信息，不进行重试
- **信号丢失**: 检测到连接断开时，尝试重新连接

### UDP Socket错误
- **端口已占用**: 记录错误并尝试使用备用端口（port + 1）
- **发送失败**: 记录错误信息，返回错误码给调用者
- **接收超时**: 使用非阻塞模式，超时后继续监听

### 数据处理错误
- **数据包过大**: 拒绝超过1472字节的数据包，记录警告
- **无效数据**: 丢弃无法解析的数据包，记录警告
- **缓冲区溢出**: 使用固定大小缓冲区，防止溢出

### 配置错误
- **环境变量缺失**: 编译时错误，提示用户设置必需的环境变量
- **IP地址格式错误**: 编译时错误，提示正确的IP格式

## Testing Strategy

### 单元测试（Unit Tests）

单元测试用于验证特定示例、边缘情况和错误条件：

1. **配置解析测试**
   - 测试有效IP地址解析（例如："192.168.1.100"）
   - 测试环境变量读取
   - 测试缺失环境变量的编译时错误

2. **边界条件测试**
   - 测试最大UDP数据包大小（1472字节）
   - 测试超大数据包拒绝（1473字节）
   - 测试空数据包处理

3. **错误处理测试**
   - 测试WiFi连接失败场景
   - 测试UDP发送失败场景
   - 测试无效数据包处理

### 属性测试（Property-Based Tests）

属性测试用于验证跨所有输入的通用属性：

1. **网络配置属性**
   - 使用快速检查库（如 `quickcheck` 或 `proptest`）
   - 生成随机有效IP地址和端口号
   - 验证配置往返一致性

2. **数据传输属性**
   - 生成随机大小的数据包（0-1472字节）
   - 生成随机内容（UTF-8文本和二进制数据）
   - 验证发送-接收往返一致性

3. **并发处理属性**
   - 生成随机数量的连续数据包
   - 验证所有数据包都被正确处理

**配置要求**:
- 每个属性测试最少运行100次迭代
- 每个测试必须引用其设计文档中的属性
- 标签格式: **Feature: udp-communication, Property {number}: {property_text}**

### 测试平衡

- 单元测试关注具体示例和边缘情况
- 属性测试通过随机化处理大量输入覆盖
- 两者互补，共同提供全面的测试覆盖

### 集成测试

由于这是嵌入式系统，集成测试需要实际硬件：

1. **WiFi连接测试**: 在真实WiFi环境中测试连接
2. **UDP通信测试**: 使用外部UDP服务器/客户端进行端到端测试
3. **长时间运行测试**: 验证系统稳定性（24小时连续运行）

## Implementation Notes

### 依赖项

基于 `demo_static_ip` 示例，需要以下依赖：

```toml
[dependencies]
blocking-network-stack = { git = "https://github.com/bjoernQ/blocking-network-stack.git", rev = "b3ecefc" }
embedded-io = "0.6.1"
esp-alloc = { path = "../../../esp-alloc" }
esp-backtrace = { path = "../../../esp-backtrace", features = ["panic-handler", "println"] }
esp-bootloader-esp-idf = { path = "../../../esp-bootloader-esp-idf" }
esp-hal = { path = "../../../esp-hal", features = ["log-04", "unstable"] }
esp-println = { path = "../../../esp-println", features = ["log-04"] }
esp-rtos = { path = "../../../esp-rtos", features = ["esp-radio", "log-04"] }
esp-radio = { path = "../../../esp-radio", features = ["log-04", "smoltcp", "unstable", "wifi"] }
smoltcp = { version = "0.12.0", default-features = false, features = ["medium-ethernet", "socket-udp"] }
```

注意：需要添加 `socket-udp` 特性到 smoltcp。

### 环境变量

必需的环境变量：
- `SSID`: WiFi网络名称
- `PASSWORD`: WiFi密码
- `STATIC_IP`: 设备静态IP地址（例如："192.168.1.100"）
- `GATEWAY_IP`: 网关IP地址（例如："192.168.1.1"）

可选的环境变量：
- `UDP_PORT`: UDP监听端口（默认：8080）
- `TARGET_IP`: UDP客户端目标IP（用于客户端模式）
- `TARGET_PORT`: UDP客户端目标端口（用于客户端模式）

### 内存分配

基于 `demo_static_ip` 示例，分配72KB堆内存：

```rust
esp_alloc::heap_allocator!(size: 72 * 1024);
```

### 主循环设计

系统将运行一个主循环，持续：
1. 调用 `stack.work()` 处理网络事件
2. 检查UDP socket是否有数据
3. 处理接收到的数据包
4. 发送响应或主动发送数据（客户端模式）

### 运行模式

支持两种运行模式：

1. **服务器模式**: 监听指定端口，接收数据包并回复
2. **客户端模式**: 定期向目标服务器发送数据包

模式选择基于环境变量：如果设置了 `TARGET_IP` 和 `TARGET_PORT`，则运行客户端模式；否则运行服务器模式。
