# Factor Analysis 编码规范

本文档归纳此项目中 Rust 后端和 Vue 前端遵循的编码约定。新代码应当保持与这些约定一致。

---

## 通用原则

1. **可读性优先**：代码面向六个月后的维护者。不堆砌抽象，不为了"将来"过度设计。
2. **无冗余**：不保留死代码、注释掉的代码、过时别名或废弃的重导出。
3. **性能有意识**：避免不必要的分配、拷贝和间接层。Rust 侧 SIMD 加速关键路径。
4. **测试覆盖边界**：测试行为、不变量和边界，不测试实现细节。

---

## Rust 后端约定

### 命名

- `snake_case`：模块、函数、变量
- `CamelCase`：类型、trait、枚举变体
- `SCREAMING_SNAKE_CASE`：常量、静态变量
- 模块目录名使用单数（`moving/` 而非 `movings/`，`basic/` 而非 `basics/`）
- 带数字后缀的模块名称使用下划线连接（`trix_n.rs`、`turnover_rate_n.rs`）
- API 路径中使用 `kebab-case`（`/api/mode1/list`）

### 模块组织

- 每个模块一个目录，`mod.rs` 作为入口，子功能拆分到独立文件
- `mod.rs` 只做重导出（`pub use`）和公共 API 声明，实现逻辑在子文件
- `pub use` 在 `mod.rs` 顶部集中排列；不同来源以空行分组
- 一个文件不超过 ~400 行；超长时拆分子模块

### 导入风格

```rust
// 标准库 → 外部 crate → 内部 crate，每组空行分隔，按字母排序
use std::{collections::HashMap, sync::Arc};

use salvo::{Router, prelude::*};
use serde::{Deserialize, Serialize};

use crate::prelude::*;
use crate::{CONFIG, DF};
```

- 优先 `use crate::{A, B}` 组合导入，避免分散的 `use crate::A; use crate::B;`

### 错误处理

- 统一使用 `Res<T>` 响应类型（位于 `src/toolbox/resp.rs`）
- 快捷宏族（按使用频率排序）：
  - `res!(data => code, "msg")` — 构建响应
  - `resolve!("data" => code, "msg")` — 返回 `Ok(Res)`
  - `reject!(code, "msg")` — 返回 `Err(Res)`
  - `resf!/resolvef!/rejectf!` — 支持格式化字符串的变体
- 标准错误类型通过 `From` 自动转换为 `Res`（见 `src/toolbox/macros/err_resp.rs`）
- 不使用 `anyhow` / `thiserror`，统一用 `io::Result` 或 `Res<T>`

### 请求参数

- 每个参数类型实现 `ArgsHandle` trait + `Default`
- 使用 `validator::Validate` derive 做校验
- 请求体提取使用 `VJson<T>`（带校验）或 `Json<T>`（无校验）
- 筛选参数通过 `Filter` 结构体传递

### 并发

- 使用 `tokio` 做异步 IO，`rayon` 做 CPU 密集并行
- `Cache::get_or_run` 通过 `spawn_blocking` 执行耗时计算，避免阻塞 async 运行时
- 相同请求的并发去重使用 `broadcast::channel`：先到者执行计算并广播结果，后续请求订阅同一 channel
- 全局状态使用 `std::sync::LazyLock` + `Arc`，启动时一次性初始化

### Cache 模式

```rust
// 每个因子 API 遵循此模式：
async fn my_factor(args: VJson<Req>) -> Resp<Arc<RawValue>> {
    let id = "factor-name";
    let cache = MODE1.cache();
    let key = ...; // args 序列化

    Ok(res!(cache.get_or_run(Arc::from(key), || run(args.0)) => 200, "ok"))
}
```

### 因子注册

```rust
// 每个因子模块提供：
pub async fn router() -> Router                   // 注册到 mode1 路由树
pub struct Req { base: Base, ... }                // 请求参数（Default + Validate）
fn register(filter: &Filter) -> (Arc<RawValue>, Receiver<Arc<RawValue>>) // 模式一列表注册
fn run(args: Req) -> Box<RawValue>               // 实际计算
```

- `Req` 必须同时实现 `Default`（提供默认参数）、`Validate`（参数校验）、`ArgsHandle`（序列化标记）
- 因子中文名称定义在 `src/router/mode1/mod.rs` 顶部的常量中

### 代码风格

- 行宽 `max_width = 150`（见 `Rustfmt.toml`）
- 使用 `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]` 顺序固定
- derive 宏放在 impl block 之上，空行分隔
- doc comment 使用 `///` 中文描述，包含末尾句号
- `#[cfg(test)]` 测试模块位于文件末尾，`tests` 函数名用 `snake_case` 描述行为

### 配置

- `config.toml` 使用 `#[serde(deny_unknown_fields)]` 拒绝未知字段
- 数据结构实现 `Default` 提供合理的默认值
- 数据路径在配置中集中管理，不硬编码

---

## Vue 前端约定

### 项目结构

```
src/
├── api/           # HTTP 请求（每个领域一个文件）
├── types/         # TypeScript 类型定义（与 API 返回对齐）
├── stores/        # Pinia store（每个关注点一个）
├── views/         # 页面级组件（路由直接引用）
├── components/
│   ├── common/    # 全局共享组件
│   └── visualization/  # ECharts 图表组件
├── features/      # 业务逻辑模块
│   └── mode1/     # 模式一相关逻辑
├── utils/         # 纯函数工具
├── router/        # Vue Router 配置
└── assets/        # 静态资源
```

### 命名

- **文件**：`PascalCase.vue`（组件）、`camelCase.ts`（脚本）、`snake_case.js`（弃用）
- **组件名**：多词 PascalCase，避免与 HTML 元素冲突
- **Pinia store**：`useXxxStore = defineStore('xxx', ...)` — store ID 用 kebab-case
- **类型**：`PascalCase` 接口名，与后端模型对齐（`ModeFilter`, `QuantileData`）

### State 管理

- 使用 Pinia **组合式 API**（`setup store` 风格），不使用选项式 API
- 每个 store 使用 `return { ... }` 显式暴露接口
- `shallowRef` 用于只整体替换的引用类型（列表），避免深层响应式开销
- 异步请求加 request version 防止竞态条件：

```typescript
let listRequestVersion = 0

async function loadList(filter: ModeFilter): Promise<void> {
  const version = ++listRequestVersion
  // ...
  const data = await fetch(...)
  if (listRequestVersion === version) items.value = data
}
```

- `reactive` 用于嵌套对象，`ref` 用于基础值和简单值

### API 层

- `api/mode1.ts` 封装 `fetch` 调用，不依赖外部 HTTP 库
- 统一从 `{ code, info, data }` 响应中提取 `data`，失败时抛出 `Error`
- 请求/响应的类型定义从 `types/` 导入
- API base URL 通过 `import.meta.env.VITE_API_BASE_URL` 配置

### Vue 组件

- `<script setup lang="ts">` — 不使用 Options API
- `defineOptions({ name: 'Xxx' })` — 显式命名组件（keep-alive 所需）
- 优先 `computed` 而非 watch，watch 只在有副作用时使用
- template 内表达式尽量简单，复杂逻辑放到 computed 或函数
- 全局 UI 组件（Loading, Message）挂载在 `App.vue` 最外层，store-driven

### 样式

- 使用 Naive UI 组件库，不额外引入 UI 框架
- 自定义样式写在 `<style scoped>` 内
- 全局样式只有 `src/assets/main.css`（最少量的重置/变量）

### ECharts 可视化

- 组件位于 `components/visualization/`，每个图表一个 `.vue` 文件
- 使用 `vue-echarts` 的 `<VChart>` 组件
- ECharts 模块在 `main.ts` 中全局注册（tree-shakeable import）
- 图表组件接收处理后的数据 props，不直接依赖 store
- `chartKeyboard.ts` 提供键盘导航支持（方向键切换分位）

### 路由

- 使用 `createWebHistory`（HTML5 History 模式）
- 路由 name 使用 kebab-case（`mode1`, `mode1-preview`）
- 使用 `KeepAlive` 缓存 `FactorDashboard` 以避免重复加载列表

### 类型

- 与后端 API 返回的 JSON 结构一一对应
- 使用 `interface` 而非 `type`（Vue 生态惯例）
- `readonly` 用于不应被修改的字段

---

## Git 提交约定

虽然不是强制约束，但提交信息推荐遵循：

```
<type>: <简短描述>

<可选详细说明>
```

type 参考：`feat` / `fix` / `refactor` / `perf` / `docs` / `style` / `test` / `chore`

---

## 测试约定

### Rust

- 测试位于每个文件末尾的 `#[cfg(test)] mod tests { ... }` 中
- 函数名用 `snake_case` 描述测试行为：`fn profit_push_defers_annualized_profit()`
- 测试命名模式：`{被测函数}_{场景}_{预期行为}`
- 使用断言宏 `assert_eq!` / `assert!`，不引入 `test-case` 等框架
- 一个 test 只验证一个关注点

### TypeScript / Vue

- 目前使用 `vue-tsc --build` 做类型检查（`pnpm type-check`）
- 逻辑函数（如 `factorSeries.ts`, `detail.ts`）可添加纯函数测试
- 组件测试当前未引入
