# NodeRS

[English](README.en.md) | 简体中文

NodeRS 是跑在 Linux 上的 Xboard / PxBoard 节点端。在服务器上执行一条命令接入面板，之后节点、用户、监听端口、证书全部由面板下发，本机只保留对接类型、API 地址、机器密钥和 `machine_id`。协议由 [Aerion](https://github.com/MoeclubM/Aerion) 提供，与 [XBClient](https://github.com/MoeclubM/XBClient) 共用同一套实现。

[![Release](https://img.shields.io/github/v/release/MoeclubM/NodeRS?style=flat-square)](https://github.com/MoeclubM/NodeRS/releases)
[![CI](https://img.shields.io/github/actions/workflow/status/MoeclubM/NodeRS/ci.yml?style=flat-square&label=CI)](https://github.com/MoeclubM/NodeRS/actions/workflows/ci.yml)
[![License](https://img.shields.io/github/license/MoeclubM/NodeRS?style=flat-square)](LICENSE)

## 特性

- 兼容 Xboard 机器模式（`/api/v2/server/*`、`/api/v2/server/machine/*`），节点增减、用户同步、流量与在线状态自动回报
- 原生对接 PxBoard 机器接口，安装时传入 `--panel-type pxboard`；省略时仍对接 Xboard
- 一机一进程，挂在同一台机器上的所有节点都由一个 NodeRS 管理
- 本地只配置面板连接信息，节点运行与证书签发由面板管理
- 证书由面板 `cert_config` 决定：文件路径、内联 PEM、Let's Encrypt HTTP-01 / DNS-01（Cloudflare、AliDNS）、本机自签
- 多用户、设备数限制和 `speed_limit` 在协议运行时生效
- Release 自带 amd64 / arm64 的 GNU（glibc 2.36+）与 musl 包，脚本自动识别架构和 libc
- 面板里协议用不到的字段打告警后忽略，缺密码、cipher 错误这类真问题才会挡启动

## 协议

| 协议 | TCP | UDP | 说明 |
| --- | :---: | :---: | --- |
| AnyTLS | ✓ | UoT | 多路复用、padding |
| Hysteria2 | ✓ | 原生 | Salamander、BBR |
| Mieru | ✓ | 原生 / 流内 | TCP 与 UDP underlay |
| Sudoku | ✓ | UoT | 独立用户 PSK、KIP、经典/packed 下行、HTTPMask legacy / WS |
| Naive | ✓ | UoT | HTTP/1.1、H2、H3 |
| Shadowsocks | ✓ | ✓ | AEAD / 2022 |
| Trojan | ✓ | 流内 | WS / H2 / gRPC / XHTTP |
| TUIC v5 | ✓ | 原生 / 流 | QUIC |
| VLESS | ✓ | ✓ | TLS、REALITY、Vision |
| VMess | ✓ | ✓ | AEAD |

协议能力边界见 [Aerion 文档](https://github.com/MoeclubM/Aerion)。

SS-2022 的 AES-128、AES-256 支持 TCP、UDP 和 UoT 多用户，按用户统计流量并执行限速、设备数限制。用户密钥优先使用面板下发的 Base64 `password`；只下发 UUID 时，按 Xboard 规则取 UUID 前 16/32 字节后进行 Base64 编码。AES 节点始终使用 `server_key:用户密钥` 连接，用户数量变化不会改变密钥格式。ChaCha20 仅支持单用户，Xboard 多用户节点请使用 AES，与官方节点的支持范围一致。

## 快速开始

在 Xboard 里创建机器，记下机器 ID 和机器密钥，把节点挂到这台机器上并配好端口、协议、证书，然后在服务器上执行：

```bash
curl -fsSL https://raw.githubusercontent.com/MoeclubM/NodeRS/main/scripts/install.sh | bash -s -- \
  --api https://api.example.com \
  --key 机器密钥 \
  --machine-id 1
```

Alpine 等没有 systemd 的系统用 [install-openrc.sh](scripts/install-openrc.sh) 代替。一台服务器对接多个面板、指定版本、自定义路径等用法见[安装文档](docs/zh/install.md)。

对接 PxBoard 时先在“机器”中取得机器编号与密钥，再安装支持 PxBoard 的 NodeRS 包：

```bash
curl -fsSL https://raw.githubusercontent.com/MoeclubM/NodeRS/main/scripts/install.sh | bash -s -- \
  --version v0.1.49 --panel-type pxboard \
  --api https://panel.example.com --key 机器密钥 --machine-id 1
```

接口、安装包要求与流量重试说明见 [PxBoard 接入](docs/zh/pxboard.md)。

安装完成后看一眼日志，节点开始监听就说明跑通了：

```bash
noders log -f
```

不带参数运行 `noders` 是交互菜单。之后节点增减、改端口、换证书都回面板操作，NodeRS 自动跟上。

## 常用命令

```bash
noders restart 1            # 只重启 machine_id = 1，不带参数则作用于全部实例
noders update               # 升级到最新 Release
noders uninstall --all      # 卸载
```

完整命令与选择器写法见[管理命令](docs/zh/management.md)。

## 文档

| 文档 | 说明 |
| --- | --- |
| [从零搭建教程](docs/zh/tutorial.md) | 从面板到节点跑通全流程 |
| [安装与升级](docs/zh/install.md) | 脚本参数、源码编译 |
| [PxBoard 接入](docs/zh/pxboard.md) | 原生接口、机器配置、流量重试 |
| [管理命令](docs/zh/management.md) | `noders` 子命令与选择器 |
| [证书配置](docs/zh/certificates.md) | `cert_config` 各模式与 DNS-01 |
| [常见问题](docs/zh/faq.md) | 节点离线、启动失败等排查 |

## 相关项目

- [Aerion](https://github.com/MoeclubM/Aerion) — 协议与 TUN 核心
- [XBClient](https://github.com/MoeclubM/XBClient) — Xboard 用户端（Android / Windows / Linux）
- [Xboard](https://github.com/cedar2025/Xboard) — 面板

## 许可

MIT。详见 [LICENSE](LICENSE)。
