# Implementation Plan: UDP Communication

## Overview

基于 `demo_static_ip` 示例实现UDP通信功能，支持服务器模式和客户端模式。实现将分为项目设置、WiFi配置、UDP服务器、UDP客户端和测试几个阶段。

## Tasks

- [x] 1. 项目结构设置和基础配置
  - 创建 `examples/myprj/udp` 目录结构
  - 创建 `Cargo.toml` 配置文件，添加所有必需的依赖项
  - 添加 `socket-udp` 特性到 smoltcp 依赖
  - 配置所有ESP32系列芯片的特性标志
  - _Requirements: 6.1, 6.2_

- [x] 2. 实现配置管理模块
  - [x] 2.1 创建配置结构体和环境变量解析
    - 定义 `UdpConfig` 结构体
    - 实现 `parse_ip()` 函数解析IP地址字符串
    - 使用 `env!()` 宏读取必需的环境变量（SSID, PASSWORD, STATIC_IP, GATEWAY_IP）
    - 使用 `option_env!()` 宏读取可选的环境变量（UDP_PORT, TARGET_IP, TARGET_PORT）
    - _Requirements: 6.1, 6.2, 6.3, 6.4_

  - [x] 2.2 编写配置解析的单元测试

    - 测试 `parse_ip()` 函数的有效IP地址解析
    - 测试边界情况（如 "0.0.0.0", "255.255.255.255"）
    - _Requirements: 6.1_

- [x] 3. 实现WiFi初始化和静态IP配置
  - [x] 3.1 创建WiFi初始化函数
    - 初始化硬件外设（esp-hal）
    - 配置堆分配器（72KB）
    - 初始化RTOS定时器
    - 初始化esp-radio控制器
    - 创建WiFi控制器和设备接口
    - _Requirements: 1.1_

  - [x] 3.2 实现WiFi连接逻辑
    - 配置WiFi客户端模式（SSID和密码）
    - 启动WiFi并执行网络扫描
    - 连接到WiFi网络
    - 实现连接状态检查循环
    - _Requirements: 1.1, 1.4_

  - [x] 3.3 实现静态IP配置
    - 创建网络接口（smoltcp interface）
    - 配置静态IP地址、网关和子网掩码
    - 使用 `blocking_network_stack` 设置IP配置
    - 验证网络配置是否生效
    - _Requirements: 1.2, 1.3_

  - [x] 3.4 编写属性测试：静态IP配置正确性

    - **Property 1: 静态IP配置正确性**
    - **Validates: Requirements 1.2, 1.3**
    - 生成随机有效的IP地址和网关配置
    - 验证配置后查询返回相同的IP地址
    - _Requirements: 1.2, 1.3_

- [x] 4. Checkpoint - 确保WiFi连接和静态IP配置正常工作
  - 确保所有测试通过，如有问题请询问用户

- [x] 5. 实现UDP服务器模式
  - [x] 5.1 创建UDP socket并绑定端口
    - 使用 smoltcp 创建UDP socket
    - 绑定到指定端口（从环境变量读取，默认8080）
    - 实现端口占用错误处理（尝试备用端口）
    - _Requirements: 2.1_

  - [x] 5.2 实现UDP数据包接收逻辑
    - 在主循环中调用 `stack.work()` 处理网络事件
    - 使用UDP socket接收数据包
    - 解析数据包内容（支持UTF-8文本和二进制数据）
    - 记录接收到的数据包信息（时间戳、大小、源地址）
    - _Requirements: 2.2, 4.1, 4.2, 4.3, 5.3_

  - [x] 5.3 实现UDP响应发送逻辑
    - 构造确认响应消息（包含原始数据的摘要）
    - 使用UDP socket发送响应到源地址
    - 记录发送的响应信息
    - _Requirements: 2.3, 5.3_

  - [x] 5.4 实现连续数据包处理
    - 在主循环中持续监听和处理数据包
    - 确保能够处理多个连续的数据包
    - 实现适当的延迟避免CPU占用过高
    - _Requirements: 2.4_

  - [x] 5.5 编写属性测试：UDP服务器端口绑定

    - **Property 2: UDP服务器端口绑定**
    - **Validates: Requirements 2.1**
    - 生成随机有效端口号（1024-65535）
    - 验证服务器能够成功绑定
    - _Requirements: 2.1_

  - [x] 5.6 编写属性测试：UDP数据包接收和回复

    - **Property 3: UDP数据包接收和回复**
    - **Validates: Requirements 2.2, 2.3**
    - 生成随机UDP数据包
    - 验证服务器能够解析并发送回复
    - _Requirements: 2.2, 2.3_

  - [ ]* 5.7 编写属性测试：连续数据包处理
    - **Property 4: 连续数据包处理**
    - **Validates: Requirements 2.4**
    - 生成随机数量的连续数据包
    - 验证所有数据包都被正确处理
    - _Requirements: 2.4_

- [x] 6. 实现UDP客户端模式
  - [x] 6.1 创建UDP客户端配置
    - 检查是否设置了 TARGET_IP 和 TARGET_PORT 环境变量
    - 如果设置，则进入客户端模式
    - 解析目标服务器地址
    - _Requirements: 3.1_

  - [x] 6.2 实现UDP数据发送逻辑
    - 创建UDP socket
    - 构造测试数据包（包含时间戳和序列号）
    - 发送数据包到目标服务器
    - 记录发送的数据包信息
    - _Requirements: 3.2, 5.3_

  - [x] 6.3 实现响应接收逻辑
    - 等待服务器响应（设置超时）
    - 接收并解析响应数据
    - 验证响应内容
    - 记录接收到的响应信息
    - _Requirements: 3.3_

  - [x] 6.4 实现客户端主循环
    - 定期发送数据包（例如每5秒）
    - 处理发送失败和超时情况
    - 记录统计信息（发送成功/失败次数）
    - _Requirements: 3.2, 3.3, 3.4_

  - [x] 6.5 编写属性测试：UDP客户端配置

    - **Property 5: UDP客户端配置**
    - **Validates: Requirements 3.1**
    - 生成随机有效的目标IP和端口
    - 验证客户端能够正确配置
    - _Requirements: 3.1_

  - [ ]* 6.6 编写属性测试：UDP数据往返一致性
    - **Property 6: UDP数据往返一致性**
    - **Validates: Requirements 3.2, 3.3, 4.1, 4.2**
    - 生成随机数据（文本和二进制）
    - 发送到服务器并接收回复
    - 验证回复包含原始数据的确认
    - _Requirements: 3.2, 3.3, 4.1, 4.2_

- [x] 7. 实现错误处理和日志记录
  - [x] 7.1 实现WiFi错误处理
    - 添加连接超时检测（30秒）
    - 实现重试逻辑（最多3次）
    - 记录详细的错误信息
    - _Requirements: 1.4, 5.1_

  - [x] 7.2 实现UDP错误处理
    - 处理端口占用错误
    - 处理发送失败错误
    - 处理接收超时
    - 处理数据包过大错误（>1472字节）
    - _Requirements: 3.4, 4.4, 5.1, 5.2_

  - [x] 7.3 实现日志记录系统
    - 使用 `esp-println` 的日志功能
    - 记录所有数据包操作（时间戳、大小、地址）
    - 实现不同日志级别（DEBUG、INFO、WARN、ERROR）
    - 记录网络状态变化
    - _Requirements: 5.1, 5.2, 5.3, 5.4_

  - [x] 7.4 编写属性测试：数据包完整性验证

    - **Property 7: 数据包完整性验证**
    - **Validates: Requirements 4.3**
    - 生成随机大小的数据包
    - 验证接收时长度与实际字节数一致
    - _Requirements: 4.3_

  - [x] 7.5 编写属性测试：数据包信息日志记录

    - **Property 8: 数据包信息日志记录**
    - **Validates: Requirements 5.3**
    - 生成随机数据包操作
    - 验证日志包含所有必需信息
    - _Requirements: 5.3_

  - [ ] 7.6 编写边缘情况测试

    - 测试最大UDP数据包大小（1472字节）
    - 测试超大数据包拒绝（1473字节）
    - 测试空数据包处理
    - 测试无效数据包处理
    - _Requirements: 4.4, 5.2_

- [x] 8. 创建主程序入口和模式选择
  - [x] 8.1 实现主函数框架
    - 初始化日志系统
    - 加载配置
    - 初始化WiFi和网络栈
    - 根据配置选择运行模式
    - _Requirements: 1.1, 6.1, 6.2_

  - [x] 8.2 实现模式选择逻辑
    - 检查 TARGET_IP 和 TARGET_PORT 是否设置
    - 如果设置，运行客户端模式
    - 否则，运行服务器模式
    - 打印当前运行模式信息
    - _Requirements: 2.1, 3.1_

  - [x] 8.3 集成所有模块
    - 连接WiFi初始化、UDP服务器/客户端、错误处理模块
    - 确保所有功能正常协作
    - 添加适当的注释和文档
    - _Requirements: 1.1, 1.2, 2.1, 3.1_

- [x] 9. 创建README文档
  - 编写项目说明文档
  - 说明如何设置环境变量
  - 提供服务器模式和客户端模式的使用示例
  - 添加故障排除指南
  - _Requirements: 6.1, 6.2, 6.3, 6.4_

- [x] 10. Final Checkpoint - 确保所有功能正常工作
  - 确保所有测试通过
  - 在实际硬件上测试WiFi连接
  - 测试UDP服务器模式（接收和回复）
  - 测试UDP客户端模式（发送和接收）
  - 测试错误处理和日志记录
  - 如有问题请询问用户

## Notes

- 标记为 `*` 的任务是可选的，可以跳过以加快MVP开发
- 每个任务都引用了具体的需求以便追溯
- Checkpoint任务确保增量验证
- 属性测试验证通用正确性属性
- 单元测试验证具体示例和边缘情况
- 由于是嵌入式系统，某些测试可能需要在实际硬件上运行
