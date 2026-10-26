# PxBoard Integration

[简体中文](../zh/pxboard.md) | [English](../en/pxboard.md)

NodeRS connects to PxBoard through its nodes plugin. The local `type` setting selects the panel API and authentication method.

This integration is distributed by the `MoeClub-Open/NodeRS` organization fork under the original author's MIT license. Release tag `v0.1.49-pxboard.1` uses executable version `0.1.49`.

## Panel Setup

Use PxBoard core 0.2.0 or later with access and nodes plugins 1.1.0 or later. Client configuration exports also require the subscriptions plugin.

Create a machine in the administration panel and obtain its machine number and key. Create nodes with the `NodeRS` engine, assign the machine, and configure their protocol, address, ports, and certificates. Users need active access accounts, available quota, and permission to use the node's groups.

One machine can host multiple nodes. Machine and node numbers are separate identifiers. Set `--api` to the panel root, such as `https://panel.example.com`, without an `/api/v1` suffix.

## Installation

Both the executable and installer must support PxBoard. From an extracted release bundle:

```bash
bash install.sh --version v0.1.49-pxboard.1 --panel-type pxboard \
  --api https://panel.example.com \
  --key MACHINE_KEY \
  --machine-id 1
```

Use `install-openrc.sh` on OpenRC systems. From a source checkout, run `bash scripts/install.sh`; it downloads a release bundle. If that bundle lacks PxBoard support, installation stops before replacing the installed executable. Use a release containing this feature or a local PxBoard-capable build.

To download the installer and pin the release:

```bash
curl -fsSL https://raw.githubusercontent.com/MoeClub-Open/NodeRS/codex/pxboard-support/scripts/install.sh | bash -s -- \
  --version v0.1.49-pxboard.1 --panel-type pxboard \
  --api https://panel.example.com --key MACHINE_KEY --machine-id 1
```

The default release repository is `MoeClub-Open/NodeRS`. To use another repository, set `NODERS_REPOSITORY=owner/repo` on the `bash` command and provide the same override on later upgrade commands.

Check executable capabilities with:

```bash
./noders --supported-panels
# xboard pxboard
```

The default `--panel-type` is `xboard`. Every `--machine` in one invocation uses that type. To connect the host to both panel types, run the installer separately for each type. The shared NodeRS executable supports both, while services and configuration files remain separate.

## Local Configuration

```toml
[panel]
type = "pxboard"
api = "https://panel.example.com"
key = "replace-with-machine-key"
machine_id = 1
```

`type` accepts `xboard` or `pxboard`; omission selects Xboard. PxBoard configuration names use `pxboard-<machineId>-<addressId>.toml` and services use `noders-pxboard-<machineId>-<addressId>`. Existing Xboard names stay unchanged.

When starting NodeRS manually, pass the configuration path and use a writable persistent working directory:

```bash
cd /var/lib/noders/anytls
/usr/local/lib/noders/noders /etc/noders/anytls/machines/ACTUAL_CONFIG.toml
```

Installed systemd and OpenRC services already set their working directory. After rotating a machine key, update the local `key` and restart the corresponding service.

## API Contract

Paths share the prefix `/api/v1/extensions/nodes/machines/{machineId}`. Requests carry the machine key in `X-Machine-Key`; successful JSON responses put their payload in `data`. Keys are never sent in the URL.

| Method and relative path | Purpose |
| --- | --- |
| `GET /handshake` | Require machine API version `1.0` |
| `GET /nodes` | Node numbers, protocols, and polling intervals |
| `GET /nodes/{nodeId}/config` | Listen, transport, security, and protocol configuration |
| `GET /nodes/{nodeId}/users` | Active users with separate UUID and connection password |
| `POST /nodes/{nodeId}/traffic` | Upload and download byte counts with a unique batch ID |
| `POST /nodes/{nodeId}/alive` | Online accounts and IPs |
| `POST /nodes/{nodeId}/status` | Node status |
| `POST /status` | Machine status |

Node configuration contains `protocol`, `host`, `listen`, `transport`, `security`, and `protocolOptions`. NodeRS adapts this structure to its existing protocol runtime; PxBoard does not need Xboard compatibility endpoints. Separate UUID and password fields support the credential requirements of Shadowsocks, Trojan, TUIC, and other protocols.

The current integration uses HTTP polling. Aerion determines protocol capabilities. Unsupported transports, routing, or fallback settings prevent that node configuration from being applied instead of silently changing its meaning. The panel must supply valid certificate and key settings where required.

## Traffic Retries and Maintenance

Before sending usage, NodeRS writes a batch to `pxboard-<encodedPanelAddress>-<machineId>-node-<nodeId>-traffic.json` in its working directory. A network failure or process restart retries the same batch ID; the nodes plugin deduplicates it. A batch is removed only after the panel acknowledges it. Files are separated by panel, machine, and node.

Keep the directory writable and provide sufficient disk space. Do not delete pending traffic files. When relocating the service, stop it and copy these files along with the rest of the state. Restore connectivity and drain pending batches before disabling a node or machine. The queue can recover usage that was collected and saved during normal operation.

```bash
noders log -f pxboard-1
noders restart pxboard-1
noders uninstall --panel-type pxboard --machine-id 1
```

A numeric service selector such as `noders restart 1` matches machine 1 across panel types. `pxboard-1` selects only PxBoard. Targeted uninstall defaults to Xboard unless `--panel-type pxboard` is supplied; `--all` removes the entire installation and its data.

Upgrade preserves configuration, certificates, and state. If PxBoard instances exist, the upgrade script rejects bundles without PxBoard support before replacing the executable.
