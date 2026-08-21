# MRCLI 第一版实施拆解

## 1. 目标范围

第一版目标：

1. 只支持 MySQL
2. 启动时读取 `~/.config/mrcli/config.json`
3. 初始态只展示本地配置中的连接
4. SQL 草稿存放于 `~/.config/mrcli/drafts/`
5. 草稿文件名使用 `YYYY-MM-DD-N.sql`
6. 编辑态使用 `tui-textarea`
7. 程序以五种状态切换运行
8. 支持执行 SQL 并展示结果
9. UI 风格参考《辐射4》哔哔小子终端

第一版明确不做：

1. PostgreSQL
2. 手动输入连接
3. 草稿标题
4. 多语句执行
5. 查询历史
6. 执行取消
7. 结果导出
8. 多标签页
9. SQL 语法高亮

---

## 2. 固定路径约定

配置文件：

- `~/.config/mrcli/config.json`

草稿目录：

- `~/.config/mrcli/drafts/`

建议项目内文档路径：

- `docs/implementation-plan.md`

---

## 3. 配置文件设计

### 3.1 文件格式

```json
{
  "default_connection": "local_mysql",
  "connections": [
    {
      "name": "local_mysql",
      "kind": "mysql",
      "host": "127.0.0.1",
      "port": 3306,
      "username": "root",
      "password": "root",
      "database": "app_db"
    }
  ]
}
```

### 3.2 校验规则

1. `connections` 不能为空
2. 每个连接的 `name` 必须非空且唯一
3. `kind` 必须是 `mysql`
4. `host` 必须非空
5. `username` 必须非空
6. `database` 必须非空
7. `port` 必须为有效端口
8. `default_connection` 可为空
9. 若 `default_connection` 不存在，则回退到第一个连接

### 3.3 错误场景

1. 配置文件不存在
2. JSON 解析失败
3. `connections` 为空
4. 某条连接字段不合法
5. `kind` 不是 `mysql`

---

## 4. 草稿文件设计

### 4.1 命名规则

草稿文件名格式：

```text
YYYY-MM-DD-N.sql
```

示例：

```text
2026-08-20-1.sql
2026-08-20-2.sql
2026-08-20-3.sql
```

### 4.2 生成规则

1. 获取当天日期字符串 `YYYY-MM-DD`
2. 扫描 `~/.config/mrcli/drafts/` 下符合当天命名规则的文件
3. 找出当天最大序号
4. 新文件使用 `最大序号 + 1`
5. 新建文件初始内容可为空

### 4.3 草稿排序规则

建议第一版按文件名倒序展示：

1. 日期新的在前
2. 同日序号大的在前

### 4.4 草稿目录策略

建议：

1. `drafts/` 不存在时自动创建
2. 只识别 `.sql` 文件
3. 删除草稿时直接删除文件
4. 保存草稿时直接覆盖当前文件

---

## 5. 状态机设计

### 5.1 状态定义

建议使用统一枚举：

- `Init`
- `QueryList`
- `QueryEdit`
- `QueryRunning`
- `ResultView`

### 5.2 状态流转

主流程：

1. `Init -> QueryList`
2. `QueryList -> QueryEdit`
3. `QueryEdit -> QueryRunning`
4. `QueryRunning -> ResultView`
5. `ResultView -> QueryList`
6. `ResultView -> QueryEdit`

补充流转：

1. `QueryList -> QueryRunning`
2. `QueryList -> Init`
3. `QueryEdit -> QueryList`

---

## 6. 页面拆解

### 6.1 初始态 `Init`

职责：

1. 加载并展示连接列表
2. 展示当前选中连接详情
3. 支持重新加载配置
4. 进入 SQL 工作区

页面布局建议：

1. 顶部标题栏：`MRCLI`
2. 左侧连接列表
3. 右侧连接详情
4. 底部快捷键栏
5. 状态栏显示错误或提示信息

快捷键：

1. `Up/Down`：切换连接
2. `Enter`：进入 `QueryList`
3. `r`：重新加载配置
4. `q`：退出程序

### 6.2 查看语句状态 `QueryList`

职责：

1. 展示草稿文件列表
2. 预览当前草稿内容
3. 新建草稿
4. 删除草稿
5. 打开草稿编辑
6. 直接执行当前草稿

页面布局建议：

1. 顶部显示当前连接
2. 左侧显示草稿列表
3. 右侧显示草稿预览
4. 底部显示快捷键

快捷键：

1. `Up/Down`：切换草稿
2. `Enter`：进入编辑态
3. `n`：新建草稿并进入编辑
4. `d`：删除当前草稿
5. `F5`：执行当前草稿
6. `Esc`：返回 `Init`

### 6.3 编辑语句状态 `QueryEdit`

职责：

1. 使用 `tui-textarea` 编辑 SQL
2. 保存当前草稿
3. 执行当前 SQL

页面布局建议：

1. 顶部显示当前连接和文件名
2. 中间为多行编辑区
3. 底部显示快捷键
4. 状态栏显示保存结果和错误提示

快捷键：

1. `Ctrl+S`：保存
2. `F5`：保存并执行
3. `Esc`：返回 `QueryList`

输入原则：

1. 普通输入交给 `tui-textarea`
2. 删除、换行、移动交给 `tui-textarea`
3. 应用层仅拦截 `Ctrl+S`、`F5`、`Esc`

### 6.4 运行语句状态 `QueryRunning`

职责：

1. 展示 SQL 执行中
2. 后台异步执行 SQL
3. 完成后自动切换到结果页

页面布局建议：

1. 中央显示 `EXECUTING`
2. 显示当前连接名
3. 显示 SQL 摘要
4. 底部可显示等待提示

快捷键：

1. 第一版不处理业务快捷键
2. 等待执行结束自动跳转

### 6.5 显示结果状态 `ResultView`

职责：

1. 展示查询结果表格
2. 展示非查询语句结果
3. 展示错误信息
4. 支持滚动浏览

页面布局建议：

1. 顶部显示连接名、耗时、结果类型
2. 中间显示表格或错误文本
3. 底部显示快捷键

快捷键：

1. `Esc`：返回 `QueryList`
2. `e`：返回 `QueryEdit`
3. `r`：重新执行
4. `Up/Down/Left/Right`：滚动结果

---

## 7. 代码结构拆解

第一版建议目录结构：

```text
src/
  main.rs
  app.rs
  config.rs
  drafts.rs
  db.rs
  event.rs
  model.rs
  ui.rs
```

### 7.1 `main.rs`

职责：

1. 初始化终端环境
2. 进入 alternate screen
3. 开启 raw mode
4. 创建 `App`
5. 驱动主循环
6. 程序退出时恢复终端状态

建议包含：

1. `main()`
2. `run_app(...)`
3. 终端恢复逻辑封装

### 7.2 `app.rs`

职责：

1. 定义 `App`
2. 管理全局状态
3. 处理状态切换
4. 分发键盘事件
5. 触发保存、执行、重载配置等行为

建议包含：

1. `AppState` 或从 `model.rs` 引入
2. `App` 结构体
3. `impl App`
4. `handle_key_event(...)`
5. `transition_to(...)`
6. `start_query_execution(...)`
7. `finish_query_execution(...)`

### 7.3 `config.rs`

职责：

1. 解析配置文件路径
2. 读取 `config.json`
3. 反序列化配置
4. 校验配置合法性
5. 计算默认选中连接索引

建议包含：

1. `load_config() -> Result<AppConfig>`
2. `config_path() -> Result<PathBuf>`
3. `validate_config(...) -> Result<()>`
4. `default_connection_index(...) -> usize`

### 7.4 `drafts.rs`

职责：

1. 解析草稿目录路径
2. 扫描草稿列表
3. 读取草稿内容
4. 新建草稿文件
5. 保存草稿文件
6. 删除草稿文件
7. 生成日期+序号文件名

建议包含：

1. `drafts_dir() -> Result<PathBuf>`
2. `ensure_drafts_dir() -> Result<PathBuf>`
3. `list_drafts() -> Result<Vec<DraftEntry>>`
4. `read_draft(path: &Path) -> Result<String>`
5. `create_new_draft() -> Result<DraftEntry>`
6. `save_draft(path: &Path, content: &str) -> Result<()>`
7. `delete_draft(path: &Path) -> Result<()>`
8. `next_draft_file_name(...) -> Result<String>`

### 7.5 `db.rs`

职责：

1. 根据连接配置构建 MySQL 连接串
2. 执行 SQL
3. 判断查询或非查询
4. 将执行结果转成统一结构

建议包含：

1. `build_mysql_url(profile: &ConnectionProfile) -> String`
2. `execute_sql(profile: &ConnectionProfile, sql: &str) -> ExecutionResult`
3. `is_query_sql(sql: &str) -> bool`
4. `execute_query(...)`
5. `execute_command(...)`
6. 行结果转字符串的辅助函数

### 7.6 `event.rs`

职责：

1. 可选地封装按键事件映射
2. 避免 `app.rs` 里匹配过于混乱

建议包含：

1. `AppEvent` 枚举
2. `map_key_event(...) -> Option<AppEvent>`

如果第一版不想拆太多，这个文件可先省略，把事件匹配直接写在 `app.rs`。

### 7.7 `model.rs`

职责：

1. 放公共数据结构
2. 放状态枚举
3. 放结果结构

建议包含：

1. `AppConfig`
2. `ConnectionProfile`
3. `DraftEntry`
4. `ExecutionResult`
5. `ResultKind`
6. `AppState`

### 7.8 `ui.rs`

职责：

1. 页面渲染
2. 主题定义
3. 布局定义
4. 状态栏、快捷键栏渲染

建议包含：

1. `draw(frame, app)`
2. `render_init(...)`
3. `render_query_list(...)`
4. `render_query_edit(...)`
5. `render_query_running(...)`
6. `render_result_view(...)`
7. `render_status_bar(...)`
8. `render_help_bar(...)`
9. 主题色辅助函数

---

## 8. 核心数据结构

### 8.1 配置结构

```rust
pub struct AppConfig {
    pub default_connection: Option<String>,
    pub connections: Vec<ConnectionProfile>,
}
```

```rust
pub struct ConnectionProfile {
    pub name: String,
    pub kind: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: String,
}
```

### 8.2 草稿结构

```rust
pub struct DraftEntry {
    pub file_name: String,
    pub path: PathBuf,
    pub content: String,
    pub modified_at: Option<SystemTime>,
}
```

### 8.3 结果结构

```rust
pub enum ResultKind {
    Query,
    Command,
    Error,
}
```

```rust
pub struct ExecutionResult {
    pub kind: ResultKind,
    pub elapsed_ms: u128,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub affected_rows: Option<u64>,
    pub error_message: Option<String>,
    pub truncated: bool,
}
```

### 8.4 状态结构

```rust
pub enum AppState {
    Init,
    QueryList,
    QueryEdit,
    QueryRunning,
    ResultView,
}
```

### 8.5 应用结构

`App` 建议至少包含以下字段：

1. `state: AppState`
2. `config: Option<AppConfig>`
3. `selected_connection_index: usize`
4. `drafts: Vec<DraftEntry>`
5. `selected_draft_index: Option<usize>`
6. `editor: TextArea<'static>`
7. `current_result: Option<ExecutionResult>`
8. `status_message: Option<String>`
9. `last_error: Option<String>`
10. `result_row_offset: usize`
11. `result_col_offset: usize`
12. `should_quit: bool`

如果需要处理异步执行，还应增加：

1. 当前执行中的 SQL 文本
2. 当前执行中的连接快照
3. 后台任务完成后的结果通道

---

## 9. 依赖清单

建议在 `Cargo.toml` 引入：

1. `ratatui`
2. `crossterm`
3. `tokio`
4. `serde`
5. `serde_json`
6. `anyhow`
7. `dirs`
8. `chrono`
9. `tui-textarea`
10. `sqlx`

建议方向：

1. `tokio` 开启运行时能力
2. `sqlx` 只开启 MySQL 所需 feature
3. 不要提前引入 PostgreSQL feature

---

## 10. 关键函数级拆解

### 10.1 `main.rs`

建议步骤：

1. 初始化终端
2. 创建 `App::new()`
3. 调用初始化加载逻辑
4. 启动事件循环
5. 在循环中：
   - 先渲染
   - 再轮询事件
   - 将事件交给 `App`
   - 检查是否退出
6. 退出时恢复终端

伪代码：

```rust
fn main() -> Result<()> {
    setup_terminal()?;
    let result = run_app();
    restore_terminal()?;
    result
}
```

### 10.2 `App::new()`

职责：

1. 初始化默认状态
2. 创建空编辑器
3. 加载配置
4. 加载草稿列表
5. 设置默认选中项
6. 若加载失败，保留错误消息并停留在 `Init`

### 10.3 `App::handle_key_event(...)`

建议按状态分发：

```rust
match self.state {
    AppState::Init => self.handle_init_key(key),
    AppState::QueryList => self.handle_query_list_key(key),
    AppState::QueryEdit => self.handle_query_edit_key(key),
    AppState::QueryRunning => self.handle_query_running_key(key),
    AppState::ResultView => self.handle_result_view_key(key),
}
```

### 10.4 `handle_init_key(...)`

处理：

1. `Up/Down`
2. `Enter`
3. `r`
4. `q`

其中 `r` 会：

1. 重新读取配置
2. 重置选中连接
3. 更新状态栏消息

### 10.5 `handle_query_list_key(...)`

处理：

1. 草稿列表移动
2. 新建草稿
3. 删除草稿
4. 打开草稿
5. 执行草稿
6. 返回初始页

关键动作：

1. `n`：创建草稿文件，刷新列表，选中新草稿，打开编辑态
2. `Enter`：读取当前草稿内容并写入编辑器
3. `F5`：读取当前草稿内容后执行

### 10.6 `handle_query_edit_key(...)`

处理逻辑优先级：

1. 先拦截 `Ctrl+S`
2. 再拦截 `F5`
3. 再拦截 `Esc`
4. 其他输入转发给 `tui-textarea`

`Ctrl+S` 步骤：

1. 读取编辑器全部文本
2. 保存到当前草稿路径
3. 刷新草稿列表
4. 更新状态栏消息

`F5` 步骤：

1. 先保存
2. 检查 SQL 是否为空
3. 切到 `QueryRunning`
4. 发起异步执行

### 10.7 `handle_result_view_key(...)`

处理：

1. `Esc`
2. `e`
3. `r`
4. 结果滚动

滚动建议：

1. `Up/Down` 控制行偏移
2. `Left/Right` 控制列偏移
3. 偏移量不允许越界

---

## 11. SQL 执行逻辑拆解

### 11.1 SQL 分类规则

第一版不做完整 SQL 解析，采用前缀判断：

查询类关键字：

1. `select`
2. `show`
3. `describe`
4. `desc`
5. `explain`
6. `with`

其他全部按非查询处理。

步骤：

1. `trim()` SQL
2. 转小写
3. 取首个关键字
4. 进行匹配

### 11.2 多语句限制

第一版建议限制为单语句执行。

实现建议：

1. 若检测到明显多个语句分隔符，可直接报错
2. 或保守一点，先不显式检测，只约定只支持单条 SQL

更稳妥的方案是主动给提示：

- `第一版暂不支持多语句执行`

### 11.3 查询结果转换

建议：

1. 查询结果统一转成字符串二维表
2. 所有列按显示文本处理
3. 遇到 `NULL` 时显示为 `NULL`
4. 二进制或复杂类型显示为占位文本或格式化字符串

### 11.4 结果集上限

建议第一版给出行数上限：

1. 最多保留前 `200` 行
2. 超出时设置 `truncated = true`
3. 结果页状态栏显示“结果已截断”

---

## 12. 结果页最小实现方案

### 12.1 查询结果

展示内容：

1. 顶部元信息：连接名、耗时、行数
2. 表格头：列名
3. 表格体：行数据
4. 底部状态：滚动提示、截断提示

最小策略：

1. 单元格内容过长时截断
2. 列太多时根据 `result_col_offset` 横向裁剪
3. 先保证“能看”，后续再优化“好看”

### 12.2 非查询结果

展示内容：

1. `OK`
2. `affected_rows`
3. `elapsed_ms`

### 12.3 错误结果

展示内容：

1. `ERROR`
2. 错误摘要
3. 详细错误文本

---

## 13. UI 主题拆解

第一版主题建议：

1. 深色背景
2. 亮绿色主文字
3. 暗绿色辅助文字
4. 选中项高亮
5. 错误信息使用偏亮对比色

统一视觉元素：

1. 顶部标题栏
2. 主内容边框
3. 底部快捷键栏
4. 状态栏

建议主题原则：

1. 不做复杂动画
2. 不做过多闪烁
3. 优先可读性
4. 保持全页面一致

---

## 14. 开发阶段拆解

### 阶段 1：项目骨架

任务：

1. 增加依赖
2. 初始化 TUI 主循环
3. 实现终端进入和恢复
4. 定义 `AppState`
5. 搭出五个占位页面

验收：

1. 程序可启动
2. 程序可退出
3. 终端不会退出后错乱
4. 页面切换可用

### 阶段 2：配置加载

任务：

1. 实现配置路径解析
2. 加载并校验 `config.json`
3. 初始态显示连接列表
4. 支持配置重载

验收：

1. 合法配置能显示
2. 非法配置能报错
3. 可从初始态进入草稿页

### 阶段 3：草稿文件管理

任务：

1. 自动创建 `drafts/`
2. 扫描 `.sql` 草稿
3. 生成日期+序号新文件名
4. 新建草稿
5. 删除草稿
6. 读取草稿

验收：

1. 草稿文件命名正确
2. 草稿列表顺序正确
3. 删除后列表能刷新

### 阶段 4：编辑器接入

任务：

1. 接入 `tui-textarea`
2. 草稿内容加载进编辑器
3. 支持保存
4. 支持返回列表

验收：

1. 文本输入正常
2. 保存后文件内容正确
3. 快捷键和输入不冲突

### 阶段 5：MySQL 执行

任务：

1. 接入 `sqlx` MySQL
2. 构建连接串
3. 实现查询和非查询执行
4. 运行态异步执行
5. 执行后写入统一结果结构

验收：

1. 可连 MySQL
2. 可执行 `SELECT`
3. 可执行 `INSERT/UPDATE/DELETE`
4. 非法 SQL 有错误信息

### 阶段 6：结果页

任务：

1. 表格展示查询结果
2. 展示影响行数
3. 展示错误信息
4. 支持上下左右滚动
5. 支持行数截断提示

验收：

1. 查询结果可浏览
2. 错误可读
3. 大结果不会直接撑坏页面

### 阶段 7：样式打磨

任务：

1. 统一绿色主题
2. 优化标题栏和状态栏
3. 优化快捷键栏
4. 优化窄终端布局

验收：

1. 风格统一
2. 键盘操作路径顺畅
3. 终端尺寸较小时仍可用

---

## 15. 测试清单

必须手工验证：

1. `config.json` 不存在
2. `config.json` 非法 JSON
3. `connections` 为空
4. `default_connection` 不存在
5. `drafts/` 不存在
6. 新建多个同日草稿后序号正确
7. 保存草稿后重启内容仍存在
8. 空 SQL 执行时有提示
9. 非法 SQL 返回错误
10. `SELECT` 结果能正常展示
11. `INSERT/UPDATE/DELETE` 影响行数正确
12. 长文本列不会撑坏布局
13. 窄终端下界面仍能使用
14. 程序退出后终端状态恢复正常

---

## 16. 风险与注意事项

1. `tui-textarea` 默认按键行为要与应用快捷键避让
2. MySQL 行值转字符串时会遇到类型兼容问题
3. 结果表格横向滚动需要尽早预留
4. SQL 执行不能阻塞 UI 主循环
5. 退出时必须恢复 raw mode 和 alternate screen
6. 草稿文件序号生成要避免重名覆盖

---

## 17. 后续扩展点

为后续 PostgreSQL 预留：

1. 保留 `ConnectionProfile.kind`
2. `ExecutionResult` 保持数据库无关
3. `db.rs` 内部预留驱动分发入口
4. UI 不直接依赖 MySQL 类型

这样第二版支持 PostgreSQL 时，主要修改数据库层，不需要大改状态机和页面层。
