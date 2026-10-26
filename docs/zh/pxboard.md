# PxBoard 接入

[简体中文](../zh/pxboard.md) | [English](../en/pxboard.md)

NodeRS 通过 PxBoard 的节点插件对接机器。PxBoard 使用自己的 API 和认证方式；本地配置中的 `type` 决定使用哪一组接口。

此功能由 `MoeClub-Open/NodeRS` 组织分支提供，保留原作者的 MIT 许可。发布标签 `v0.1.49-pxboard.1` 使用程序版本 `0.1.49`。

## 面板准备

需要 PxBoard 核心 0.2.0 或更新版本，以及 1.1.0 或更新版本的访问账户、节点管理插件。用于导出客户端配置时还需要订阅插件。

在管理后台的“机器”中创建机器，取得机器编号与接入密钥。添加节点时选择 `NodeRS`，指定所属机器，填写协议、服务器地址、端口与证书。用户需要有正常的访问账户、剩余流量和对应节点分组权限。

一台机器可以挂载多个节点；节点编号和机器编号由面板分配，不能互换。`--api` 填面板根地址，例如 `https://panel.example.com`，不要附加 `/api/v1`。

## 安装

安装包中的程序和安装脚本都必须包含 PxBoard 支持。在已解压的安装包目录运行：

```bash
bash install.sh --version v0.1.49-pxboard.1 --panel-type pxboard \
  --api https://panel.example.com \
  --key 机器接入密钥 \
  --machine-id 1
```

OpenRC 系统将脚本名换成 `install-openrc.sh`。从源码目录运行时使用 `bash scripts/install.sh`，脚本会下载 Release 包；若该包尚不支持 PxBoard，安装会在替换程序前停止。需要发布包含此功能的安装包，或者使用本机已有的 PxBoard 构建产物。

直接下载安装脚本时可固定此发布标签：

```bash
curl -fsSL https://raw.githubusercontent.com/MoeClub-Open/NodeRS/codex/pxboard-support/scripts/install.sh | bash -s -- \
  --version v0.1.49-pxboard.1 --panel-type pxboard \
  --api https://panel.example.com --key 机器接入密钥 --machine-id 1
```

默认发布源为 `MoeClub-Open/NodeRS`。使用其他发布仓库时，在 `bash` 命令前设置 `NODERS_REPOSITORY=owner/repo`，后续升级命令也需显式提供相同覆盖值。

程序本身可以检查支持的面板类型：

```bash
./noders --supported-panels
# xboard pxboard
```

`--panel-type` 默认是 `xboard`，同一次安装命令中所有 `--machine` 使用这一类型。需要同时接入两种面板时分别执行安装命令；共享的 NodeRS 程序同时支持两种类型，配置与服务独立保存。

## 本地配置

```toml
[panel]
type = "pxboard"
api = "https://panel.example.com"
key = "replace-with-machine-key"
machine_id = 1
```

`type` 取 `xboard` 或 `pxboard`；省略时按 Xboard 处理。PxBoard 配置文件名为 `pxboard-<机器编号>-<地址标识>.toml`，服务名为 `noders-pxboard-<机器编号>-<地址标识>`。Xboard 原有实例命名不变，两类配置不会相互覆盖。

手动启动时传入配置路径，并将工作目录设为可写的持久目录：

```bash
cd /var/lib/noders/anytls
/usr/local/lib/noders/noders /etc/noders/anytls/machines/实际配置文件.toml
```

安装器创建的 systemd / OpenRC 服务已经配置工作目录。轮换机器密钥后，更新本地 `key` 并重启对应服务。

## 接口约定

所有路径均以 `/api/v1/extensions/nodes/machines/{machineId}` 开头。请求使用 `X-Machine-Key` 头传递机器密钥，响应数据放在 JSON 的 `data` 字段中。密钥不出现在 URL 中。

| 方法与相对路径 | 内容 |
| --- | --- |
| `GET /handshake` | 确认机器接口版本为 `1.0` |
| `GET /nodes` | 节点编号、协议及同步间隔 |
| `GET /nodes/{nodeId}/config` | 监听、传输、安全与协议配置 |
| `GET /nodes/{nodeId}/users` | 有效用户及独立的 UUID / 连接密码 |
| `POST /nodes/{nodeId}/traffic` | 带唯一批次编号的上传与下载字节数 |
| `POST /nodes/{nodeId}/alive` | 当前在线账户和 IP |
| `POST /nodes/{nodeId}/status` | 节点运行状态 |
| `POST /status` | 主机运行状态 |

节点配置由 `protocol`、`host`、`listen`、`transport`、`security`、`protocolOptions` 组成。NodeRS 将该结构转换为已有协议运行配置，不需要 PxBoard 提供 Xboard 兼容接口。用户的连接密码与 UUID 分开接收，适用于 Shadowsocks、Trojan、TUIC 等不同凭据需求。

当前使用 HTTP 轮询。协议能力由 Aerion 决定；不支持的传输、路由和回落设置会阻止该节点配置应用，避免静默改变配置含义。面板应下发该协议需要的有效证书与密钥。

## 流量重试与维护

每次回报先将批次写入工作目录中的 `pxboard-<面板地址编码>-<机器编号>-node-<节点编号>-traffic.json`，再发送给面板。网络失败或程序重启后使用原批次编号重试；节点插件按批次去重。收到面板确认后才从队列删除批次。不同面板、机器与节点的队列分开保存。

工作目录需要可写且有足够空间。不要手动删除待发送的流量文件。移动运行目录或迁移服务器时，停止服务后一起复制这些文件；在停用节点或机器前先恢复网络并让待发送批次完成回报。只有正常运行时采集并写入队列的流量才能由队列恢复。

常用管理命令：

```bash
noders log -f pxboard-1
noders restart pxboard-1
noders uninstall --panel-type pxboard --machine-id 1
```

数字选择器 `noders restart 1` 会选择所有面板类型中编号为 1 的机器；使用 `pxboard-1` 只选择 PxBoard。卸载命令不传 `--panel-type` 时只按 Xboard 查找指定机器，`--all` 则删除整个 NodeRS 安装及数据。

升级时保留配置、证书和工作目录。若已有 PxBoard 实例，升级器会拒绝缺少 PxBoard 支持的安装包，避免将正在使用的实例升级到不支持其面板的程序。
