# UDP Communication Example

基于ESP32系列芯片的UDP通信示例，支持服务器模式和客户端模式，使用静态IP配置。

## 项目概述

本项目演示了如何在ESP32系列芯片上实现UDP通信功能，包括：

- ✅ WiFi连接与静态IP配置
- ✅ UDP服务器模式（接收数据包并回复）
- ✅ UDP客户端模式（发送数据包并接收响应）
- ✅ 支持UTF-8文本和二进制数据
- ✅ 完整的错误处理和日志记录
- ✅ 基于环境变量的灵活配置

## 系统架构

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

## 支持的芯片

- ESP32
- ESP32-C2
- ESP32-C3
- ESP32-C6
- ESP32-S2
- ESP32-S3

## 环境变量配置

### 必需的环境变量

这些环境变量必须在编译前设置，否则会导致编译错误：

| 变量名 | 说明 | 示例 |
|--------|------|------|
| `SSID` | WiFi网络名称 | `"MyWiFiNetwork"` |
| `PASSWORD` | WiFi密码 | `"mypassword123"` |
| `STATIC_IP` | 设备的静态IP地址 | `"192.168.1.100"` |
| `GATEWAY_IP` | 网关IP地址 | `"192.168.1.1"` |

### 可选的环境变量

这些环境变量有默认值，可以根据需要覆盖：

| 变量名 | 说明 | 默认值 | 示例 |
|--------|------|--------|------|
| `UDP_PORT` | UDP监听端口（服务器模式） | `8080` | `"9000"` |
| `TARGET_IP` | 目标服务器IP（客户端模式） | 无（服务器模式） | `"192.168.1.100"` |
| `TARGET_PORT` | 目标服务器端口（客户端模式） | 无（服务器模式） | `"8080"` |
| `ESP_LOG_LEVEL` | 日志级别 | `INFO` | `"DEBUG"` |

### 日志级别说明

- `ERROR`: 仅显示错误信息
- `WARN`: 显示警告和错误
- `INFO`: 显示信息、警告和错误（默认）
- `DEBUG`: 显示所有消息，包括调试信息

## 运行模式

系统根据环境变量自动选择运行模式：

### 服务器模式（默认）

当 `TARGET_IP` 和 `TARGET_PORT` **未设置**时，系统运行在服务器模式：

- 绑定到指定的UDP端口（默认8080）
- 监听传入的UDP数据包
- 解析数据包内容（支持文本和二进制）
- 向发送方回复确认消息
- 持续处理多个数据包

### 客户端模式

当 `TARGET_IP` 和 `TARGET_PORT` **已设置**时，系统运行在客户端模式：

- 定期向目标服务器发送UDP数据包（每5秒）
- 等待并验证服务器响应
- 跟踪统计信息（成功/失败次数）
- 处理超时和错误情况

## 使用示例

### 示例1: UDP服务器模式

在一台ESP32设备上运行UDP服务器：

```bash
# 设置环境变量
export SSID="MyWiFi"
export PASSWORD="mypassword"
export STATIC_IP="192.168.1.100"
export GATEWAY_IP="192.168.1.1"
export UDP_PORT="8080"

# 编译并烧录（以ESP32-C3为例）
cargo build --release --features esp32c3
cargo espflash flash --release --features esp32c3 --monitor
```

服务器将监听端口8080，等待接收UDP数据包。

### 示例2: UDP客户端模式

在另一台ESP32设备上运行UDP客户端：

```bash
# 设置环境变量
export SSID="MyWiFi"
export PASSWORD="mypassword"
export STATIC_IP="192.168.1.101"
export GATEWAY_IP="192.168.1.1"
export TARGET_IP="192.168.1.100"
export TARGET_PORT="8080"

# 编译并烧录（以ESP32-C3为例）
cargo build --release --features esp32c3
cargo espflash flash --release --features esp32c3 --monitor
```

客户端将每5秒向服务器（192.168.1.100:8080）发送测试数据包。

### 示例3: 使用电脑作为UDP服务器

您也可以在电脑上运行UDP服务器来测试ESP32客户端：

```bash
# 使用netcat作为UDP服务器（Linux/Mac）
nc -u -l 8080

# 或使用Python脚本
python3 -c "
import socket
sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
sock.bind(('0.0.0.0', 8080))
print('UDP server listening on port 8080...')
while True:
    data, addr = sock.recvfrom(1472)
    print(f'Received from {addr}: {data.decode(\"utf-8\", errors=\"ignore\")}')
    sock.sendto(b'ACK', addr)
"
```

然后配置ESP32为客户端模式，指向电脑的IP地址。

### 示例4: 使用电脑作为UDP客户端

您也可以从电脑向ESP32服务器发送数据：

```bash
# 使用netcat发送UDP数据包（Linux/Mac）
echo "Hello ESP32" | nc -u 192.168.1.100 8080

# 或使用Python脚本
python3 -c "
import socket
sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
sock.sendto(b'Hello ESP32', ('192.168.1.100', 8080))
data, addr = sock.recvfrom(1472)
print(f'Response: {data.decode(\"utf-8\", errors=\"ignore\")}')
sock.close()
"
```

## 编译和烧录

### 前置要求

1. 安装Rust工具链：
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. 安装espflash工具：
```bash
cargo install espflash
```

3. 安装目标架构（根据您的芯片选择）：
```bash
# ESP32 (Xtensa)
rustup target add xtensa-esp32-none-elf

# ESP32-C3 (RISC-V)
rustup target add riscv32imc-unknown-none-elf

# ESP32-C6 (RISC-V)
rustup target add riscv32imac-unknown-none-elf
```

### 编译步骤

根据您的ESP32芯片型号选择相应的特性标志：

```bash
# ESP32
cargo build --release --features esp32

# ESP32-C2
cargo build --release --features esp32c2

# ESP32-C3
cargo build --release --features esp32c3

# ESP32-C6
cargo build --release --features esp32c6

# ESP32-S2
cargo build --release --features esp32s2

# ESP32-S3
cargo build --release --features esp32s3
```

### 烧录到设备

```bash
# 烧录并打开串口监视器
cargo espflash flash --release --features esp32c3 --monitor

# 仅烧录
cargo espflash flash --release --features esp32c3

# 指定串口
cargo espflash flash --release --features esp32c3 --port /dev/ttyUSB0
```

## 数据格式

### UDP数据包限制

- 最大UDP负载大小：**1472字节**
  - 以太网MTU: 1500字节
  - IP头: 20字节
  - UDP头: 8字节
  - 可用负载: 1472字节

- 超过1472字节的数据包将被拒绝并记录警告

### 支持的数据类型

1. **UTF-8文本数据**：
   - 系统会尝试将接收到的数据解析为UTF-8文本
   - 如果解析成功，将在日志中显示文本内容

2. **二进制数据**：
   - 如果数据不是有效的UTF-8，将作为二进制数据处理
   - 日志中显示数据包大小和十六进制摘要

### 服务器响应格式

服务器收到数据包后，会发送确认响应：

```
ACK: Received N bytes from IP:PORT
```

其中：
- `N`: 接收到的字节数
- `IP:PORT`: 发送方的地址和端口

## 日志输出示例

### 服务器模式日志

```
=== UDP Communication System Starting ===
Hardware initialized with CPU clock: 240MHz
Heap allocator configured: 72KB
Configuration loaded:
  SSID: MyWiFi
  Static IP: 192.168.1.100
  Gateway: 192.168.1.1
  UDP Port: 8080
  Mode: Server
Initializing WiFi and network stack...
Connecting to WiFi network 'MyWiFi'...
WiFi connection established successfully
Configuring static IP...
Network configuration complete
=== System Initialization Complete ===
=== Starting UDP Server Mode ===
Listening on port 8080
UDP server bound to port 8080
[Server] Waiting for UDP packets...
[Server] Received 12 bytes from 192.168.1.101:54321
[Server] Content: "Hello ESP32"
[Server] Sent 42 bytes response to 192.168.1.101:54321
```

### 客户端模式日志

```
=== UDP Communication System Starting ===
Hardware initialized with CPU clock: 240MHz
Heap allocator configured: 72KB
Configuration loaded:
  SSID: MyWiFi
  Static IP: 192.168.1.101
  Gateway: 192.168.1.1
  UDP Port: 8080
  Mode: Client
  Target: 192.168.1.100:8080
Initializing WiFi and network stack...
Connecting to WiFi network 'MyWiFi'...
WiFi connection established successfully
Configuring static IP...
Network configuration complete
=== System Initialization Complete ===
=== Starting UDP Client Mode ===
Target server: 192.168.1.100:8080
[Client] Sending packet #1 (28 bytes)
[Client] Sent 28 bytes to 192.168.1.100:8080
[Client] Received 42 bytes response
[Client] Response: "ACK: Received 28 bytes from 192.168.1.101:54321"
[Client] Statistics - Success: 1, Failed: 0
```

## 故障排除

### 问题1: 编译时提示环境变量未设置

**错误信息**：
```
error: environment variable `SSID` not defined at compile time
```

**解决方案**：
确保在编译前设置所有必需的环境变量：
```bash
export SSID="YourWiFiName"
export PASSWORD="YourPassword"
export STATIC_IP="192.168.1.100"
export GATEWAY_IP="192.168.1.1"
```

### 问题2: WiFi连接失败

**日志信息**：
```
FATAL: Failed to connect to WiFi: ConnectionFailed
```

**可能原因和解决方案**：

1. **SSID或密码错误**：
   - 检查环境变量中的SSID和PASSWORD是否正确
   - 注意SSID区分大小写

2. **WiFi信号弱**：
   - 将ESP32设备移近WiFi路由器
   - 检查WiFi天线连接

3. **WiFi频段不兼容**：
   - ESP32仅支持2.4GHz WiFi，不支持5GHz
   - 确保路由器启用了2.4GHz频段

4. **路由器设置问题**：
   - 检查路由器是否启用了MAC地址过滤
   - 检查路由器是否达到最大连接数限制

### 问题3: 静态IP配置失败

**日志信息**：
```
FATAL: Failed to configure static IP
```

**可能原因和解决方案**：

1. **IP地址冲突**：
   - 确保STATIC_IP没有被网络中其他设备使用
   - 尝试使用不同的IP地址

2. **IP地址格式错误**：
   - 确保IP地址格式正确：`"192.168.1.100"`
   - 每个数字范围：0-255

3. **网关地址错误**：
   - 确保GATEWAY_IP是您路由器的实际IP地址
   - 通常是 `192.168.1.1` 或 `192.168.0.1`

4. **子网不匹配**：
   - 确保STATIC_IP和GATEWAY_IP在同一子网
   - 例如：IP `192.168.1.100` 和网关 `192.168.1.1` 在同一子网

### 问题4: UDP端口绑定失败

**日志信息**：
```
FATAL: Failed to bind UDP server: PortInUse
```

**解决方案**：

1. **更改端口号**：
   ```bash
   export UDP_PORT="9000"  # 使用不同的端口
   ```

2. **检查端口范围**：
   - 使用1024-65535范围内的端口
   - 避免使用系统保留端口（0-1023）

### 问题5: 客户端无法接收服务器响应

**日志信息**：
```
[Client] No response received (timeout)
```

**可能原因和解决方案**：

1. **服务器未运行**：
   - 确保目标服务器正在运行并监听指定端口
   - 使用 `netstat` 或 `ss` 命令检查端口状态

2. **防火墙阻止**：
   - 检查服务器防火墙设置
   - 允许UDP流量通过指定端口

3. **网络不可达**：
   - 使用 `ping` 命令测试网络连通性
   - 确保客户端和服务器在同一网络或可路由

4. **TARGET_IP或TARGET_PORT错误**：
   - 检查环境变量中的目标地址是否正确
   - 确保端口号匹配

### 问题6: 数据包过大被拒绝

**日志信息**：
```
[WARN] Packet too large (1500 bytes), maximum is 1472 bytes
```

**解决方案**：

1. **减小数据包大小**：
   - 将数据分割成多个小于1472字节的数据包
   - 实现分片和重组逻辑（如果需要）

2. **使用TCP协议**：
   - 如果需要传输大量数据，考虑使用TCP而不是UDP
   - TCP会自动处理分片和重组

### 问题7: 设备频繁重启或崩溃

**可能原因和解决方案**：

1. **堆内存不足**：
   - 当前配置：72KB堆内存
   - 如果需要更多内存，修改 `main.rs` 中的堆分配大小

2. **栈溢出**：
   - 减少局部变量的使用
   - 避免在栈上分配大型数组

3. **看门狗超时**：
   - 确保主循环不会长时间阻塞
   - 添加适当的延迟和yield点

### 问题8: 日志输出不完整或乱码

**解决方案**：

1. **调整波特率**：
   - 默认波特率：115200
   - 在串口监视器中设置相同的波特率

2. **增加日志缓冲区**：
   - 如果日志输出过快，可能会丢失
   - 考虑降低日志级别或减少日志输出

3. **检查USB连接**：
   - 使用质量好的USB线缆
   - 避免USB集线器，直接连接到电脑

### 调试技巧

1. **启用DEBUG日志**：
   ```bash
   export ESP_LOG_LEVEL="DEBUG"
   ```
   这将显示更详细的调试信息。

2. **使用串口监视器**：
   ```bash
   cargo espflash monitor
   ```
   实时查看设备输出。

3. **使用Wireshark抓包**：
   - 在电脑上运行Wireshark
   - 过滤UDP流量：`udp.port == 8080`
   - 分析数据包内容和流向

4. **检查网络配置**：
   ```bash
   # 在电脑上检查路由表
   ip route  # Linux
   route print  # Windows
   
   # 检查ARP表
   arp -a
   ```

5. **测试网络连通性**：
   ```bash
   # Ping ESP32设备
   ping 192.168.1.100
   
   # 使用nmap扫描端口
   nmap -sU -p 8080 192.168.1.100
   ```

## 性能特性

- **WiFi连接超时**：30秒
- **WiFi重试次数**：最多3次
- **UDP数据包最大大小**：1472字节
- **客户端发送间隔**：5秒
- **堆内存分配**：72KB
- **CPU时钟频率**：最大（240MHz for ESP32）

## 技术栈

- **硬件抽象层**：esp-hal
- **WiFi驱动**：esp-radio
- **网络协议栈**：smoltcp (blocking-network-stack)
- **日志系统**：esp-println
- **RTOS**：esp-rtos
- **内存分配**：esp-alloc

## 项目结构

```
examples/myprj/udp/
├── Cargo.toml              # 项目配置和依赖
├── README.md               # 本文档
└── src/
    ├── main.rs             # 主程序入口
    ├── lib.rs              # 库入口（用于测试）
    ├── config.rs           # 配置管理模块
    ├── wifi.rs             # WiFi初始化和配置
    ├── udp_server.rs       # UDP服务器实现
    └── udp_client.rs       # UDP客户端实现
```

## 需求映射

本实现满足以下需求：

- **Requirements 1.1-1.4**: WiFi连接和静态IP配置
- **Requirements 2.1-2.4**: UDP服务器功能
- **Requirements 3.1-3.4**: UDP客户端功能
- **Requirements 4.1-4.4**: 数据格式和协议
- **Requirements 5.1-5.4**: 错误处理和日志记录
- **Requirements 6.1-6.4**: 配置管理

## 测试

项目包含属性测试（Property-Based Tests）来验证核心功能：

```bash
# 运行所有测试
cargo test --features esp32c3

# 运行特定测试
cargo test --features esp32c3 test_parse_ip
```

测试覆盖：
- IP地址解析
- 配置验证
- 数据包大小限制
- 错误处理逻辑

## 许可证

本项目遵循与esp-hal相同的许可证。

## 贡献

欢迎提交问题报告和改进建议！

## 相关资源

- [esp-hal文档](https://github.com/esp-rs/esp-hal)
- [esp-radio文档](https://github.com/esp-rs/esp-radio)
- [smoltcp文档](https://docs.rs/smoltcp/)
- [ESP32系列芯片数据手册](https://www.espressif.com/en/products/socs)

## 更新日志

### v0.0.0 (初始版本)
- ✅ 实现WiFi连接和静态IP配置
- ✅ 实现UDP服务器模式
- ✅ 实现UDP客户端模式
- ✅ 支持文本和二进制数据
- ✅ 完整的错误处理和日志记录
- ✅ 基于环境变量的配置管理
