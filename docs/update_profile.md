# 更新订阅 (Update Profile)

## 支持的类型

为了代码精简，demotui 只支持**将 token 嵌入 URL** 的方式更新订阅链接。
不再有 `ProfileType::Github` / `ProfileType::GitLab` 等独立 token 字段的类型。

### URL 类型 (ProfileType::Url)

将 token 直接嵌入 URL 即可访问私有仓库的订阅文件。

**GitLab 私有仓库：**

```
https://gitlab.com/api/v4/projects/<project_id>/repository/files/Clash%2F<uuid>%2Fconfig.yaml/raw?ref=<branch>&private_token=<your_token>
```

- `project_id`：项目设置 → 通用 → 项目 ID
- `<uuid>`：用于隐藏文件路径的随机 UUID
- `ref`：分支名（默认 main）
- `private_token`：个人访问令牌，范围选择 `read_repository`

**GitHub 私有仓库：**

```
https://<token>@raw.githubusercontent.com/<user>/<repo>/<branch>/<path>
```

或：

```
https://x-access-token:<token>@raw.githubusercontent.com/<user>/<repo>/<branch>/<path>
```

- `token`：Personal Access Token (classic)，勾选 `repo` 权限

> **安全提示**：虽然 URL 会暴露 token，但 GitLab 的 `read_repository` 令牌无法列出仓库文件树。
> 使用 UUID 目录可以防止猜测文件路径。如担心泄露，可创建专用账号。

### 本地文件 (ProfileType::File)

不可更新。如需更新，重新导入并覆盖。

### 模板生成 (ProfileType::Generated)

从模板重新生成配置。

## 子资源提取

更新 `Url` 类型 profile 时，会自动提取 YAML 中的 `proxy-provider` 和 `rule-provider` 并下载到 `clash_config_dir` 目录。

如果子资源 URL 也需要 token 认证，将 token 嵌入该 URL 即可。
