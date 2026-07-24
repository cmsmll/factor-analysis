# Factor Analysis 架构文档

## 项目概要

股票因子分析服务。从本地 TBF（天软二进制格式）原始数据解析行情、财务和元数据，存入 SQLite；服务启动时全部加载到内存 `DataFrame`，通过 REST API 对外提供分位因子分析能力。配套 Vue 3 前端展示因子列表和可视化图表。

---

## 整体架构

```
┌─────────────────────────────────────────────────────┐
│                  Rust Backend                        │
│  ┌──────────┐  ┌──────────┐  ┌───────────────────┐ │
│  │  Parse    │  │   Run    │  │   Test            │ │
│  │ (TBF→DB)  │  │(Web Svr) │  │(DB integrity)     │ │
│  └────┬─────┘  └────┬─────┘  └───────────────────┘ │
│       │             │                               │
│       ▼             ▼                               │
│  ┌──────────────────────────────────────────────┐  │
│  │              DataFrame (内存)                  │  │
│  │  MarketData + Finance + Metadata per contract │  │
│  └──────────────────────────────────────────────┘  │
│       │                                             │
│       ▼                                             │
│  ┌──────────────────────────────────────────────┐  │
│  │            Mode1 (分位分析引擎)               │  │
│  │  basic / technical / momentum / emotion / risk│  │
│  └───────────────────┬──────────────────────────┘  │
│                      │                              │
│                      ▼                              │
│  ┌──────────────────────────────────────────────┐  │
│  │           Salvo HTTP Router                   │  │
│  │  /api/period, /api/indice, /api/sector        │  │
│  │  /api/mode1/list, /api/mode1/{id}             │  │
│  │  /swagger-ui (OpenAPI)                        │  │
│  └───────────────────┬──────────────────────────┘  │
└──────────────────────┼─────────────────────────────┘
                       │ HTTP JSON
                       ▼
┌─────────────────────────────────────────────────────┐
│                 Vue 3 Frontend                       │
│  ┌────────┐ ┌──────────┐ ┌──────────────────────┐  │
│  │Pinia   │ │ Vue      │ │ ECharts              │  │
│  │Stores  │ │ Router   │ │ 6 visualization      │  │
│  │        │ │          │ │ components            │  │
│  └────────┘ └──────────┘ └──────────────────────┘  │
│  UI: Naive UI Components                            │
└─────────────────────────────────────────────────────┘
```

---

## 项目目录结构

```
factor-analysis/
├── Cargo.toml              # Rust 项目配置（edition 2024）
├── config.toml             # 服务器配置、分析周期、数据路径
├── src/                    # Rust 后端
│   ├── main.rs             # 入口：mimalloc 全局分配器 + tokio main
│   ├── lib.rs              # 模块导出 + 全局懒加载常量 (CONFIG, DF, MODE1)
│   ├── app/                # CLI 子命令（parse / run / test）
│   ├── args.rs             # 请求参数类型（Filter, NumArg, IntArg...）
│   ├── cache.rs            # 文件缓存 + 广播通道并发控制
│   ├── config.rs           # TOML 配置加载与校验
│   ├── db/                 # SQLite 数据库层
│   │   ├── parse.rs        # TBF 二进制解析
│   │   ├── market.rs       # 行情数据查询
│   │   ├── finance.rs      # 财务数据查询
│   │   ├── metadata.rs     # 合约元数据查询
│   │   ├── dataframe.rs    # 内存 DataFrame 结构体
│   │   └── sql/            # 预编译 SQL 模板
│   ├── math/               # 数学/技术指标计算
│   │   ├── sum.rs          # SIMD 加速求和 (AVX2/AVX512)
│   │   ├── avg.rs          # 多模式平均值
│   │   └── moving/         # 移动技术指标（MA, MACD, BBI, CCI, TRIX, MASS...）
│   ├── model.rs            # Profit 等核心数据模型
│   ├── router/             # HTTP 路由
│   │   ├── mod.rs          # 系统级路由（hello, period, indice, sector）
│   │   └── mode1/          # 模式一：分位因子分析
│   │       ├── mod.rs      # 路由注册 + 公共参数 Base
│   │       ├── manager.rs  # Mode1Manager：因子注册/调度
│   │       ├── basic/      # 基础因子（market_value）
│   │       ├── emotion/    # 情绪因子（volume, turnover, turnover_rate）
│   │       ├── momentum/   # 动量因子（pvt, trix_n）
│   │       ├── risk/       # 风险因子（amplitude）
│   │       └── technical/  # 技术因子（bbi, bias, cci, ema, macd, mass, sma）
│   └── toolbox/            # 通用工具
│       ├── resp.rs         # 统一 JSON 响应类型 Res<T>
│       ├── macros/         # res!, resolve!, reject! 宏族
│       ├── extractor/      # Salvo 请求提取器（Json, VJson, Query...）
│       ├── serde/          # 自定义序列化（date_format）
│       └── logger/         # HTTP 请求日志中间件
├── client/                 # Vue 3 前端
│   ├── src/
│   │   ├── main.ts         # 入口：ECharts 全局注册 + Pinia + Router
│   │   ├── App.vue         # 根组件：RouterView + 全局 UI 组件
│   │   ├── router/         # 路由定义（/mode1, /mode1/:id）
│   │   ├── stores/         # Pinia 状态管理
│   │   │   ├── mode1.ts          # 因子列表 + 周期
│   │   │   ├── mode1Preview.ts   # 因子详情（per-factor 缓存）
│   │   │   ├── globalLoading.ts  # 全局加载状态
│   │   │   ├── globalMessage.ts  # 全局消息通知
│   │   │   └── globalFilterSelector.ts  # 多选弹窗
│   │   ├── api/            # HTTP 客户端（fetch 封装）
│   │   ├── types/          # TypeScript 类型定义
│   │   ├── views/          # 页面组件
│   │   │   ├── FactorDashboard.vue  # 因子列表页
│   │   │   └── Mode1Preview.vue     # 因子详情页
│   │   ├── components/
│   │   │   ├── common/     # 全局组件（Loading, Message, FilterSelector）
│   │   │   └── visualization/  # ECharts 可视化组件
│   │   ├── features/       # 业务逻辑
│   │   │   └── mode1/detail.ts  # 因子详情数据加工
│   │   └── utils/          # 工具函数
│   │       ├── factorSeries.ts   # 因子计算（收益率、夏普、回撤等）
│   │       └── chartKeyboard.ts  # 图表键盘导航
│   └── vite.config.ts      # Vite 配置（代理 /api → 127.0.0.1:7878）
├── data/                   # 运行时数据的软链接/目录占位
└── cache/                  # API 结果缓存目录
    ├── mode1/              # 模式一缓存子目录
    └── mode1-check/        # 测试缓存子目录
```

---

## 核心数据流

### 1. 数据管道：TBF → SQLite → DataFrame

```
TBF 原始文件                    SQLite 数据库                  内存 DataFrame
(market/*.tbf)  ──parse──►  market/<code>.db  ──query──►  Vec<Contract>
(finance/*.tbf) ──parse──►  finance/<code>.db ──query──►    ├── market: Vec<MarketData>
(metadata/*)    ──parse──►  metadata.db       ──query──►    ├── finance: Vec<Finance>
                                                              └── meta: Metadata
```
- `factor-analysis parse`：读取 TBF 二进制文件，按合约代码创建独立 SQLite 数据库
- 服务启动 (`App::Run`)：`DataFrameDb::from_config()` → 遍历所有合约数据库 → 加载到 `DataFrame`
- `DataFrame` 类型 (src/db/dataframe.rs)：
  - `list: Vec<Arc<Contract>>` — 每只股票的完整时序数据
  - `index: Vec<Date>` — 市场的完整交易日历
  - `sector / indice` — 板块和指数的去重集合

### 2. 因子分析请求：API → Mode1Manager → 因子函数

```
客户端 POST /api/mode1/list
  │  body: Filter { start, end, sector, indice, filter_bz, filter_st }
  ▼
Mode1Manager::execute(&filter)
  │  并行执行所有注册的因子 (tokio::spawn_blocking)
  ├── market_value(&filter)  ──► Mode1Data
  ├── bbi(&filter)           ──► Mode1Data
  ├── volume(&filter)        ──► Mode1Data
  └── ... (30+ 因子)
  │
  ▼
返回 Vec<ListItem> { args, data }
```

每个因子接口：
1. `POST /api/mode1/{id}` 独立调用
2. 通过 `Cache::get_or_run` 防重复计算（相同 args 共享结果）
3. 计算结果 `Mode1Data` 含分位收益序列、换手率、因子值等

### 3. 前端数据流

```
FactorDashboard                    Mode1Preview
  │  onMounted                      │
  ├─ fetchPeriods()                 ├─ 从 store 取 item.args
  ├─ POST /api/mode1/list           ├─ POST /api/mode1/{id}
  │  (含 filter)                    │  (含完整 ModeRequest)
  ▼                                 ▼
Mode1Store (items[])              Mode1PreviewStore (results{})
  │                                 │
  └─ 表格展示 (NaiveUI NDataTable)  └─ buildDetail() → 6 ECharts charts
                                       ├─ GroupNavChart (分位净值)
                                       ├─ IcChart (IC)
                                       ├─ IndustryIcChart (行业IC)
                                       ├─ DecayChart (收益衰减)
                                       ├─ TurnoverChart (换手率)
                                       └─ FactorMetrics (指标表)
```

---

## 核心模块详解

### db/ — SQLite 数据库层

- **每只股票一个独立数据库文件**，按股票代码命名
- 行情表 (`daily`)：日期、开盘/收盘/最高/最低价、成交量/额
- 财务表 (`finance`)：日期、总市值、流通市值、PE/PB/PS 等 18 项指标
- 元数据表 (`metadata`)：股票代码、名称、上市日期、板块、指数归属等
- 预编译 SQL 模板位于 `src/db/sql/`，运行时读取并参数化
- 使用 `rusqlite` + `bundled` 模式，无需外部 SQLite

### math/ — SIMD 加速数学库

- `sum::sum_simd`：运行时检测 CPU 特性，优先 AVX512，回退 AVX2，最后标量
- `moving/` 模块：纯 Rust 实现常见技术指标，不依赖第三方 TA 库
  - `ma.rs` — 简单移动平均 / 加权移动平均 / 指数移动平均
  - `macd.rs` — MACD 指标
  - `bbi.rs` — 多空指标（BBI）
  - `cci.rs` — 商品通道指标
  - `trix.rs` — TRIX 指标
  - `mass.rs` — MASS 指标（梅斯线）
  - `high_low.rs` — 最高价最低价统计

### cache/ — 请求缓存

- 基于文件系统的缓存：每个唯一 `args` 序列化哈希后作为文件名
- 使用 `tempfile` 原子写入防止部分写入
- `broadcast::channel` 实现并发请求去重：相同请求同时到达时，只有一个执行计算，其他订阅同一个 `Receiver`
- 缓存目录结构：`cache/mode1/<hash>`，`cache/mode1-check/<hash>`

### router/mode1/ — 分位因子分析引擎

**注册机制：** 每个因子模块提供 `pub async fn router() -> Router` 和 `pub struct Req`，通过 `enum_dispatch` 风格手动注册到 `Mode1Manager`。

**Mode1Manager：** 
- `register::<T: ArgsHandle + Default>()` 注册因子
- `execute(filter)` 并行执行所有因子，返回 `Vec<ListItem>`
- `execute_one(id, params)` 执行单个因子

**Mode1Data 数据结构：**
```
struct Mode1Data {
    name: String,           // 因子中文名
    info: String,           // 因子的描述信息
    count: u8,              // 分位数（3/5/10）
    factor: Vec<Vec<f64>>,  // 每期各分位的因子值
    profit[1-4]: Vec<Profit>, // 四种收益模式
    turnover_rate: Vec<Vec<f64>>, // 每期换手率
    datetime: Vec<Date>,    // 时间轴
}
```

### toolbox/resp.rs — 统一响应格式

所有 API 返回统一 JSON 结构：
```json
{ "code": 200, "info": "ok", "data": ... }
```
通过 `Res<T>` 类型 + `Writer` trait 实现自动序列化。错误码从 `io::Error`、`ParseError`、`ParseIntError` 自动转换。

### 全局状态 (src/lib.rs)

```rust
static CONFIG: LazyLock<Config>      // 配置，惰性加载
static DF: LazyLock<DataFrame>       // 全量市场数据，服务启动时加载
static MODE1: LazyLock<Mode1Manager>  // 因子管理器，服务启动时初始化
```

---

## 技术栈

| 层 | 技术 | 版本 |
|---|---|---|
| 运行时 | Rust (edition 2024) | nightly |
| Web 框架 | Salvo (git rev) | — |
| OpenAPI | salvo-oapi + Swagger UI | — |
| 数据库 | SQLite via rusqlite (bundled) | 0.40 |
| 序列化 | serde + serde_json | 1.0 |
| 并发 | tokio (full) + rayon + crossbeam-channel | — |
| SIMD | 运行时 CPU 特性检测 (AVX2/AVX512) | — |
| 内存分配器 | mimalloc | 0.1 |
| 前端框架 | Vue 3 | ^3.5 |
| 状态管理 | Pinia | ^3.0 |
| 路由 | Vue Router | ^5.0 |
| UI 组件 | Naive UI | ^2.44 |
| 图表 | ECharts + vue-echarts | ^6.1 |
| 前端构建 | Vite 8 + vue-tsc 3 | — |

---

## CLI 命令

```
factor-analysis parse        解析 TBF 原始数据 → SQLite（行情/财务/元数据）
factor-analysis run          启动 Web 服务（可选 -c 清空缓存）
factor-analysis test         检查数据库完整性，导出 JSON
```

## API 端点

| 方法 | 路径 | 描述 | 缓存 |
|---|---|---|---|
| GET | `/api/hello` | 健康检查 | 否 |
| GET | `/api/period` | 分析周期列表 | 否 |
| GET | `/api/indice` | 指数列表 | 否 |
| GET | `/api/sector` | 行业板块列表 | 否 |
| POST | `/api/mode1/list` | 按筛选条件获取所有因子分位值 | 是 |
| POST | `/api/mode1/{id}` | 获取单个因子指定参数的详情 | 是 |
| POST | `/api/test` | 固定参数测试接口 | 是 |
| GET | `/api-doc/openapi.json` | OpenAPI 文档 | 否 |
| GET | `/swagger-ui/` | Swagger UI | 否 |

---

## 运行方式

```bash
# 1. 解析 TBF 原始数据
cargo run --release -- parse

# 2. 启动 Web 服务（自动加载 DB）
cargo run --release -- run

# 3. 前端开发
cd client && pnpm dev

# 4. 前端生产构建
cd client && pnpm build
```

开发时 Vite 代理 `/api` 请求到 `127.0.0.1:7878`。
