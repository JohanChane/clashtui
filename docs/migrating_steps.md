# demotui 迁移步骤

> 原则：先搭框架并用 Demo 数据验证，再逐步迁移业务代码。

## 第一阶段：Demo 数据驱动（当前）

用假数据验证整个框架的流程，不引入任何真实业务逻辑。

### 1.1 Tab<C> 泛型容器完善

**当前状态**：Tab 通过 `enum_dispatch!` 宏实现，每个 Tab 是一个独立 struct。

**目标**：实现 `Tab<C: TabContent>` 泛型容器。

```rust
/// 核心 trait，每个具体 Tab 实现此 trait
trait TabContent {
    type Key: TryFrom<KeyEvent>;
    fn title(&self) -> &'static str;
    fn handle_key_event(&mut self, key: Self::Key, tasks: &mut JoinSet<TaskClosure>, state: &mut TabState);
    fn render(&mut self, f: &mut Frame, area: Rect, state: &TabState);
}
```

**Demo 验证**：创建 `DemoTabA` 和 `DemoTabB`，分别展示不同的列表数据。

### 1.2 事务系统（JoinSet + 闭包）

**目标**：按键事件生成异步任务，完成后通过闭包修改 Tab 状态。

**Demo 验证**：模拟网络请求延迟（`tokio::time::sleep`），观察：
- 处理中动画（Tab 标题 `-/|\` 循环）
- 结果返回后正确更新列表
- 错误情况的展示

```
DemoTabA-/     ← 正在模拟请求（标题动画）
DemoTabA       ← 请求完成，状态更新
```

### 1.3 三层关键事件路由

**目标**：PopUp → App → Tab，无返回值。

**Demo 验证**：
- 弹出 PopUp 时，按键不会穿透到 Tab
- App 层 `q` 退出，`Tab`/`数字` 切换 Tab
- PopUp 关闭后按键正确路由到 Tab

### 1.4 PopUp 系统

**目标**：支持以下类型，通过 oneshot 返回结果。

| 类型 | 用途 | Demo 场景 |
|------|------|-----------|
| `Input` | 文本输入 | 添加一个项目名称 |
| `Choice` | 多选一 | 选择操作类型（编辑/删除/复制） |
| `MultiChoice` | 多选多 | 批量选择要删除的项目 |
| `Confirm` | 确认/取消 | 删除确认 |
| `Msg` | 纯消息 | 操作成功/失败提示 |

**Demo 验证**：在每个 Demo Tab 中触发不同类型的 PopUp，验证 oneshot 通道正确传回结果。

### 1.5 双栏布局（7:3）

**目标**：`DualTab` 组件，左侧 70% 右侧 30%。

**Demo 验证**：左侧显示列表，右侧显示选中项的详情。

```
┌──────────────────────────┬──────────────┐
│ Item 1                   │ Name: Item 1 │
│ Item 2                   │ Status: OK   │
│ * Item 3  (selected)     │ Tags: ...    │
│ Item 4                   │              │
│ Item 5                   │              │
└──────────────────────────┴──────────────┘
```

### 1.6 内联状态显示

**目标**：不依赖弹窗显示状态，在列表项前加标记。

| 标记 | 含义 |
|------|------|
| `*` 或 `>` | 当前选中项 |
| `-` `\` `|` `/` 循环 | 该项正在处理中 |
| `!` | 该项有错误 |
| ` ` (空格) | 空闲项 |

**Demo 验证**：模拟批量操作，观察每个项目的状态标记实时更新。

### 1.7 主题系统

**目标**：YAML 主题文件，支持自定义颜色。

**Demo 验证**：`--load-theme-realtime` 参数启用热加载，修改 `theme.yaml` 后 UI 实时变化。

### 1.8 按键映射（KeyMap）

**目标**：`keymap.yaml` 定义按键绑定，通过 agent 系统加载。

**Demo 验证**：修改 keymap 中的某个按键，重启后生效。

### 1.9 鼠标支持

**目标**：点击 Tab 标题切换 Tab，点击列表项选中。

**Demo 验证**：在支持的终端中点击操作。

---

## 第二阶段：迁移业务代码

第一阶段全部验证通过后，才迁移 ClashTUI 业务逻辑。

### 2.1 数据准备

- [ ] 将 ClashTUI 的配置结构（`ConfigFile`、`BasicInfo` 等）迁移到 demotui 的 `config/` 模块
- [ ] 适配新的 `Tab<C>` 泛型模式

### 2.2 功能迁移顺序

按复杂度从低到高：

| 顺序 | 功能 | 对应旧代码 |
|------|------|-----------|
| 1 | Mihomo RESTful API 封装 | `api/` crate |
| 2 | StatusTab | 服务状态、运行模式、端口信息 |
| 3 | ClashSrvCtlTab | systemctl 服务控制 |
| 4 | ProfileTab | 订阅导入/更新/选择 |
| 5 | TemplateTab | 配置模板系统 |
| 6 | ProxiesTab | 代理节点管理（新增） |
| 7 | ConnectTab | 连接管理（新增） |

### 2.3 每个功能的迁移流程

1. 理解旧代码的行为和边界条件
2. 在 demotui 中用 Demo 数据先搭建 UI 骨架
3. 接入真实 API 替换 Demo 数据
4. 回归测试旧 ClashTUI 确认行为一致

---

## 第三阶段：抽象为通用框架

业务代码稳定后：

1. 提取 `Tab<C>`、`PopUp`、`DualTab` 等通用组件
2. 编写框架使用文档和示例
3. 将 demotui 作为该框架的参考实现保留
