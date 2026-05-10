# mihomo vs sing-box: Command Comparison

## 1. Config Validation

| Core     | Command                                              |
| -------- | ---------------------------------------------------- |
| mihomo   | `mihomo -t -d <config_dir> -f <config_path> [-m]`   |
| sing-box | `sing-box check -c <config_path>`                    |

mihomo uses flat flags (`-t` for test, `-d` for config dir, `-f` for config file) with an optional `-m` for geodata mode.
sing-box uses a `check` subcommand with a single `-c` flag. There is no geodata mode equivalent.

## 2. Service Control

| Core     | Service name           | Operation                         |
| -------- | ---------------------- | --------------------------------- |
| mihomo   | `clash_service_name`   | `systemctl start/stop/restart`    |
| sing-box | `singbox_service_name` | `systemctl start/stop/restart`    |

Command format is identical. The difference is the service name — dispatched via `Service.clash_service_name` vs `Service.singbox_service_name`.

## 3. Config Hot-Reload

| Core     | Method                                                        |
| -------- | ------------------------------------------------------------- |
| mihomo   | REST API `PUT /configs` with full YAML body                  |
| sing-box | Not supported via REST API. Send `SIGHUP` to sing-box process. |

## 4. Log Level Switching

| Core     | Method                                                                      |
| -------- | --------------------------------------------------------------------------- |
| mihomo   | REST API `PATCH /configs` modifying `log-level` field.                      |
| sing-box | Not supported via REST API. Requires editing the JSON config file + SIGHUP. |

In the Settings tab, "Switch Log Level" is greyed out when the active core is sing-box.

## 5. Capability / Permission Setting

| Core     | Command                                                         |
| -------- | --------------------------------------------------------------- |
| mihomo   | `setcap cap_net_admin,cap_net_bind_service=+ep <mihomo_bin>`   |
| sing-box | `setcap cap_net_admin,cap_net_bind_service=+ep <sing-box_bin>`  |

Identical mechanism — only the binary path differs.

## 6. REST API PATCH Support

| Field      | mihomo                 | sing-box               |
| ---------- | ---------------------- | ---------------------- |
| `mode`     | PATCH supported        | PATCH supported        |
| `log-level`| PATCH supported        | Not supported          |
| Others     | PATCH supported        | Not supported (read-only) |

Only `mode` can be changed at runtime via REST API for sing-box. All other config changes require editing the JSON config file and sending SIGHUP.

## 7. Proxy Provider

| Core     | Proxy Provider Support                                    |
| -------- | --------------------------------------------------------- |
| mihomo   | Supported (profiles can include `proxy-provider` stanzas) |
| sing-box | Not supported (profiles must contain explicit outbounds)  |
