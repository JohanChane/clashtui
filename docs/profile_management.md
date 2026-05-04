# Profile Management

## 概述

Profile 管理有两个维度：**元数据**（数据库）和 **YAML 内容**（文件）。两者通过 profile name 关联。

核心数据结构：

| 类型 | 所在 | 包含 |
|------|------|------|
| `Profile` | `clashtui.db` | `name` + `dtype` + `no_pp` |
| `ProfileData` | `clashtui.db` | `dtype` + `no_pp`（数据库存储值） |
| `LocalProfile` | 内存 | `name` + `dtype` + `path` + `content`（完整 YAML） |

管理入口：`src/functions/file/profile.rs`（核心逻辑）、`src/config/database.rs`（数据库）、`src/tui/tab/files/profile.rs`（TUI 界面）。

## Profile Model

| 字段 | 类型 | 说明 |
|------|------|------|
| `name` | `String` | profile 唯一标识，对应 `profile_yamls/<name>.yaml` |
| `dtype` | `ProfileType` | 来源类型：`File` 或 `Url(url)` |
| `no_pp` | `bool` | 更新时是否移除 `proxy-providers`/`rule-providers` 段（默认 `false`） |

### ProfileType

- **`File`** — 本地文件。来源：模板生成、本地导入。YAML 文件存放在 `profile_yamls/<name>.yaml`
- **`Url(url)`** — 远程订阅。更新时从 URL 下载最新的 YAML，保存到 `profile_yamls/<name>.yaml`，行为等价于 `File`

> 旧的 `Template` / `Generated` 类型已废弃，读取时自动迁移为 `File`。

## Storage

Profile 信息分两层存储，**name 是两者之间的唯一关联键**：

```
data/
├── clashtui.db              # ProfileManager 数据库，记录 name → (dtype, no_pp)
├── config.yaml              # Clash 路径（clash_bin_path, clash_config_path 等）
├── basic_clash_config.yaml  # 基础配置（API 地址、端口、secret 等），激活时 merge 进 profile
└── profile_yamls/
    ├── my_profile.yaml      # 实际的 Clash 配置 YAML
    ├── another.yaml
    └── ...
```

- **`clashtui.db`**（YAML）：存储 profile 元数据（类型、`no_pp` 标记、当前选中的 profile）。由 `ProfileManager` 管理，通过 `Mutex` 保护并发访问，`pm!()` 宏获取锁。`ProfileManager.current_profile` 记录当前激活的 profile。
- **`profile_yamls/`**：每个 profile 对应的 YAML 文件，文件名固定为 `<name>.yaml`，由 profile name 决定。**始终保存原始订阅内容**——`no_pp` 标记不会修改这些文件，嵌入操作仅在激活时发生在内存中，最终输出到 `clash_config_path`。

**这就是为什么不支持改名**：name 变更后 `profile_yamls/<name>.yaml` 路径对不上，需要同时重命名文件和更新数据库，容易出错。如需"改名"，创建新 profile 后删除旧的即可。

---

## 关键流程

### 激活 (Select / Apply / Enter) — 完整过程

**触发**：在 Profile 列表上按 `Enter` 或 `a`。

这是让一个 profile "生效"的过程——把它设成 Clash 当前使用的配置，并通知 Clash 重载。

**步骤**（`select()` 函数，`src/functions/file/profile.rs:128`）：

1. **加载 profile** — 调用 `profile.load_local_profile()`
   - 从数据库读取该 profile 的元数据（`name`, `dtype`）
   - 读取 `profile_yamls/<name>.yaml` 文件内容，解析为 `serde_yml::Mapping`
   - 构造 `LocalProfile { name, dtype, path: 源文件路径, content: Some(解析后的 YAML) }`
   - **注意**：`profile_yamls/<name>.yaml` 保存的是原始订阅内容，永远不会被 `no_pp` 标记修改

2. **应用 no_pp**（仅当 `profile.no_pp == true` 时） — 调用 `update_profile_without_pp()`
   - 从内容中**移除** `proxy-providers` / `rule-providers` 段
   - 对每个 provider：先检查 `<clash_config_dir>/<path>` 文件是否已存在，存在则直接读取；不存在时才并行下载，下载后保存到该文件的 `path` 以供后续复用
   - 将代理节点**嵌入** `proxies` 段，规则内容**嵌入** `rules` 段
   - 将 `proxy-groups` 中的 `use` 引用**解析为具体代理名**
   - 此操作**仅修改内存中的数据**，不写回 `profile_yamls/`，只影响最终输出到 `clash_config_path` 的配置

3. **合并基础配置** — 调用 `lprofile.merge(&load_basic()?)`
   - 读取 `data/basic_clash_config.yaml`，解析为 `Mapping`
   - 将基础配置的每一项写入 `LocalProfile.content`：
     - **普通值**（标量、Mapping）：直接覆盖
     - **Sequence 值**：将基础配置的同名 Sequence **拼接**到 profile 的后面（profile 的值在前，基础配置的值在后）
   - 这样 `basic_clash_config.yaml` 里的 `external-controller`、端口、`secret` 等会覆盖进 profile，确保 API 可用

4. **设置输出路径** — `lprofile.path = cfg.clash_config_path`
   - 从 `config.yaml` 的 `basic.clash_config_path` 读取（Clash 实际读取的配置文件路径）

5. **写盘** — `lprofile.sync_to_disk()`
   - 把合并后的 YAML 写入 `clash_config_path`

6. **更新数据库** — `db::set_current(profile)`
   - 将 `ProfileManager.current_profile` 设为该 profile 的 name，并持久化到 `clashtui.db`
   - 之后 TUI 列表会在该 profile 前显示 `*` 标记

7. **通知 Clash 重载** — `restful::config::reload(&cfg.clash_config_path)`
   - 向 Clash 的 REST API 发 `PUT /configs?force=true`，body 为 `{"path": "<clash_config_path>", "payload": ""}`
   - Clash 收到后会重新读取配置文件，应用新规则和代理

### 更新 (Update) — 完整过程

**触发**：按 `u`（更新当前选中）、`a u`（更新全部）。

`profile_yamls/<name>.yaml` **始终保存原始订阅内容**，更新时不做 `no_pp` 处理；`no_pp` 的嵌入操作仅在激活（Select）时发生。

**步骤**（`update_profile()` 函数，`src/functions/file/profile.rs:90`）：

#### Url 类型的前置步骤

1. 从数据库中记录的 URL **下载**最新的 YAML（`restful::download::profile()`）
2. 解析下载内容，验证是合法 YAML
3. 覆盖写入 `profile_yamls/<name>.yaml`

#### 所有类型的后续步骤

4. 从磁盘读取 YAML 内容，解析为 `Mapping`
5. 调用 `fetch_net_resource_statuses()`：提取 YAML 中的 `proxy-providers` 和 `rule-providers` 的 URL，并行下载并保存到 `<clash_config_dir>/<path>`，检查可访问性
6. 原样写回 YAML（格式规范化，**不做结构性修改**）
7. 弹窗显示各网络资源的可达性状态

### 模板的 Enter 过程

模板 Tab 上按 `Enter` → 触发 `Generate` 动作：
1. 弹出 Input 窗口让用户输入 profile 名（默认 `<template_name>.generated`）
2. 调用 `apply_template()`：解析模板 YAML，用模板引擎生成完整的代理配置
3. 写入 `profile_yamls/<name>.yaml`
4. 在数据库中登记为 `ProfileType::File`
5. Profile 列表自动刷新（`sync!()`）

---

## Other Operations

### 创建 / 导入

| 操作 | 方式 | 结果 |
|------|------|------|
| 新建 URL 订阅 | CLI / TUI 输入 URL | 创建 `Url` 类型，首次更新时下载 YAML |
| 导入本地文件 | CLI / TUI 选择文件 | 复制 YAML 到 `profile_yamls/`，创建 `File` 类型 |
| 应用模板 | TUI 选中模板按 Enter | 生成 YAML 到 `profile_yamls/<name>.generated.yaml`，创建 `File` 类型 |

> 所有方式创建时 `no_pp` 默认 `false`。

### Toggle no_pp

按 `N` 切换 profile 的 `no_pp` 标记。列表显示 `|nopp` 表示已开启。

`no_pp` 仅影响**激活**（Select）时的行为：
- **开启**：激活时优先从 `<clash_config_dir>/<path>` 读取已缓存文件；文件不存在时下载，下载后保存到该路径。然后将内容嵌入最终输出的 `clash_config_path`
- **关闭**：激活时原样使用 `profile_yamls/` 中的内容，保留 `proxy-providers`/`rule-providers` 段让 Clash 自行拉取

> `profile_yamls/<name>.yaml` **始终保存原始订阅内容**，不会被 `no_pp` 修改。

### Delete

按 `d` 删除 profile：移除 `profile_yamls/<name>.yaml`（忽略 NotFound），从数据库中删除记录。

### Edit

按 `e` 在 `$EDITOR`（默认 `open`/`start`，可配置 `config.yaml` 中的 `edit_cmd`）中打开 `profile_yamls/<name>.yaml`。

### Test

按 `t` 用 `clash -t -d <config_dir> -f <profile_path>` 检查配置语法是否正确。

### Preview

按 `p` 弹窗显示 profile 的完整 YAML 内容。

### Url 类型支持 Token

GitLab / GitHub 私有仓库的 token 直接嵌入 URL 即可：

**GitLab:**
```
https://gitlab.com/api/v4/projects/<id>/repository/files/.../raw?ref=main&private_token=<token>
```

**GitHub:**
```
https://<token>@raw.githubusercontent.com/<user>/<repo>/<branch>/<path>
```

---

## TUI Key Bindings (Profiles Tab)

| 键 | 操作 |
|----|------|
| `Enter` | 激活 profile（Select） |
| `u` | 更新当前 profile |
| `a u` | 批量更新所有 profile |
| `i` | 新建 URL 订阅 |
| `I` | 导入本地文件 |
| `d` | 删除 profile |
| `e` | 编辑 YAML |
| `N` | 切换 `no_pp` |
| `t` | 测试配置 |
| `p` | 预览 YAML 内容 |
| `/` | 搜索过滤 |
| `g g` | 跳到顶部 |
| `g e` | 跳到底部 |

---

## 子资源认证

如果 YAML 中 `proxy-provider` 或 `rule-provider` 的 URL 也需要 token 认证，将 token 嵌入该 URL 即可，与 Url 类型的认证方式一致。

## 子资源缓存

`proxy-provider` / `rule-provider` 下载后保存到 `<clash_config_dir>/<path>`（即 provider YAML 中声明的 `path` 字段指向的路径）。后续操作（如激活时的 `no_pp`）会优先从此缓存读取，避免重复下载。
