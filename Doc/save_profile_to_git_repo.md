# 使用 git 存放 profile

## 使用 gitlab 私有仓库存放个人的 profile

如果没有私人服务器, 可以将通过模板生成的 profile 上传到 gitlab 的私人仓库。这样就相当于个人的订阅链接。当要更换 `proxy-provider` 时, 用模板重新生成 profile 再上传到 gitlab。然后 mihomo 客户端更新该订阅链接即可。

操作如下:
1.  创建 gitlab 的私有仓库。
2.  创建[个人的访问令牌](https://gitlab.com/-/user_settings/personal_access_tokens)。令牌的范围选择 `read_repository`。
3.  配置 profile 的路径:
    -   [生成一个 uuid](https://www.uuidgenerator.net/)。
    -   在仓库中创建目录 `Clash/<uuid>`。 将你的链接放在该目录下。
    -   同理, 如果共享你的订阅链接, 你可以将 profile 放置在 `Clash` 目录下。或者放置在另外一个 uuid 目录。
4.  profile 的 url: `https://gitlab.com/api/v4/projects/<project_id>/repository/files/Clash%2F<uuid>%2Fconfig.yaml/raw?ref=<branch>&private_token=<your token>`
    -   project_id: 项目设置->通用->项目ID
    -   uuid: 刚才生成的 uuid。
    -   branch: 默认为 main
    -   your token: 个人的访问令牌
    -   文件路径的 `/`: `%2F`。

解释上面的操作:
-   url 虽然会泄露个人令牌。但是令牌的范围是 `read_repository`。该范围是无法列出仓库的文件树(文件名称)。
-   所以创建 uuid 目录, 可以防止别人通过猜测其他文件的路径, 从而通过私人令牌取得其他文件。
-   如果害怕泄露个人令牌, 可以单独创建一个专门用于共享订阅链接的 gitlab 帐号。

## 使用 github 私有仓库存放个人的 profile

添加 `Personal access tokens`:

```
Settings -> Developer settings -> Personal access tokens -> Tokens (classic) -> Generate new token -> Select scopes (check `repo`)
```

The Url of file in private repo:

```
# the url of raw file in private repo
https://raw.githubusercontent.com/xxx

# Add token
https://<token>@raw.githubusercontent.com/xxx

# OR
https://x-access-token:<token>@raw.githubusercontent.com/xxx
```
