# HTTP API 接口说明

本文档描述 `src/router` 当前注册的全部 HTTP 接口。默认监听地址来自 `config.toml` 的 `[server]`，默认值为 `http://127.0.0.1:7878`。


## OpenAPI 与 Swagger UI

服务启动后提供自动生成的 OpenAPI 3.1 文档：

| 用途 | 地址 |
| --- | --- |
| OpenAPI JSON | `/api-doc/openapi.json` |
| Swagger UI | `/swagger-ui` |

Swagger UI 按“系统”“基础数据”“模式一”“测试”分组展示接口。模式一的动态因子 ID 会以当前进程实际注册的路径出现在文档中，请求体和响应体 schema 由 Rust 数据结构自动生成。

## 通用响应

业务接口统一使用以下 JSON 包络：

```json
{
  "info": "ok",
  "code": 200,
  "data": {}
}
```

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `info` | string | 响应说明。 |
| `code` | integer | HTTP 状态码。 |
| `data` | any | 接口数据；无错误数据时通常为 `null`。 |

服务允许任意来源、方法和请求头跨域访问，但没有开启跨域凭证。

## 接口概览

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| `GET` | `/` | 服务连通性检查。 |
| `GET` | `/api/indice` | 股票池指数列表。 |
| `GET` | `/api/sector` | 股票池行业板块列表。 |
| `GET` | `/api/test` | 固定参数的测试换手率分析。 |
| `POST` | `/api/mode1/list` | 按 Filter 并发计算全部模式一因子。 |
| `POST` | `/api/mode1/{factor_id}` | 执行对应的模式一因子分析。 |

## 动态因子 ID

模式一接口使用请求类型的 `TypeId` 生成动态 `factor_id`。该值可能随类型定义或编译环境变化，客户端不应写死，应先调用：

```text
POST /api/mode1/list
```

再使用模板中的 `base.id` 请求：

```text
POST /api/mode1/{factor_id}
```

## GET /

服务连通性检查。

```json
{
  "info": "ok",
  "code": 200,
  "data": "Hello World"
}
```

## GET /api/indice

返回所有合约元数据 `indice` 的并集：

```json
{
  "info": "ok",
  "code": 200,
  "data": ["沪深300", "中证500"]
}
```

内部使用 `HashSet`，数组顺序不固定。

## GET /api/sector

返回所有合约元数据 `SW1`、`SW2`、`SW3` 的非空值并集：

```json
{
  "info": "ok",
  "code": 200,
  "data": ["银行", "证券", "半导体"]
}
```

内部使用 `HashSet`，数组顺序不固定。

## GET /api/test

使用 `2025-01-01` 至 `2025-12-31` 和 5 个分位执行固定换手率分析，结果使用缓存键 `test`。成功响应的 `data` 与模式一因子返回的 `QuantileData` 相同。

后台任务失败时返回：

```json
{
  "info": "获取数据失败",
  "code": 400,
  "data": null
}
```

## POST /api/mode1/list

请求体为 `Filter`。接口使用同一筛选条件并发计算全部已注册因子，并返回每个
因子的实际请求参数 `args` 和分析结果 `data`。当前注册顺序为：

1. 换手率因子。
2. 振幅因子。
3. 总市值因子。

请求示例：

```json
{
  "start": "2025-01-01",
  "end": "2025-12-31",
  "filter_bz": false,
  "filter_st": false,
  "sector": [],
  "indice": []
}
```

响应中的每个列表项结构如下：

```json
{
  "args": {
    "base": {
      "id": "<factor_id>",
      "count": 5,
      "filter": {}
    }
  },
  "data": {
    "name": "因子名称",
    "count": 5
  }
}
```

## POST /api/mode1/{factor_id}

请求头必须包含：

```text
Content-Type: application/json
```

请求体直接使用 `/api/mode1/list` 中对应模板：

```json
{
  "base": {
    "id": "<factor_id>",
    "count": 5,
    "filter": {
      "start": "2025-01-01",
      "end": "2025-12-31",
      "filter_bz": false,
      "filter_st": false,
      "sector": ["银行"],
      "indice": ["沪深300"]
    }
  }
}
```

### 请求字段

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `base.id` | string | 动态接口 ID，也参与缓存键计算；当前不会校验是否与 URL 一致。 |
| `base.count` | integer | 分位数量，调用方应保证大于等于 1。 |
| `base.filter.start` | string | 开始日期，格式 `YYYY-MM-DD`，超过数据范围时会裁剪。 |
| `base.filter.end` | string | 结束日期，格式 `YYYY-MM-DD`，超过数据范围时会裁剪。 |
| `base.filter.filter_bz` | boolean | 为 `true` 时排除北京证券交易所。 |
| `base.filter.filter_st` | boolean | 为 `true` 时排除名称中包含 `ST` 的合约。 |
| `base.filter.sector` | string[] | 匹配合约的 `SW1`、`SW2`、`SW3`。 |
| `base.filter.indice` | string[] | 匹配合约所属指数。 |

`sector` 与 `indice` 均为空时不进行元数据过滤；任意一个非空时，两类条件使用并集，匹配任一条件即可保留。

## 换手率因子

使用 `/api/mode1/list` 返回的第一个 `base.id`。

因子值为当日行情的 `turnover_rate`，每日从低到高排序并切分为 `base.count` 个分位。

## 振幅因子

使用 `/api/mode1/list` 返回的第二个 `base.id`。

振幅计算公式：

```text
(当日最高价 - 当日最低价) / 当日最低价
```

当日最低价为 `0` 时，振幅直接记为 `0`。每日按振幅从低到高排序并切分为 `base.count` 个分位。

## 总市值因子

使用 `/api/mode1/list` 返回的第三个 `base.id`。

总市值因子直接使用对齐财务数据中的字段：

```text
finance.total_market
```

`total_market` 单位为元。每日按总市值从低到高排序并切分为 `base.count`
个分位，不再使用总股本和当日收盘价重新计算。

## 公共分位逻辑

每个交易日执行：

1. 仅保留同时具有当日、下一交易日和下下交易日行情的股票。
2. 按对应因子值从低到高排序。
3. 使用整数边界切分为 `base.count` 个分位。
4. 计算各分位的平均因子与四种平均收益。
5. 股票数量少于分位数量时，所有分位共享当日完整股票集合。

四种收益：

| 字段 | 买入价格 | 卖出价格 | 公式 |
| --- | --- | --- | --- |
| `profit1` | 当日收盘价 | 下一交易日收盘价 | `(next.close - current.close) / current.close` |
| `profit2` | 下一交易日开盘价 | 下一交易日收盘价 | `(next.close - next.open) / next.open` |
| `profit3` | 下一交易日开盘价 | 下下交易日开盘价 | `(next2.open - next.open) / next.open` |
| `profit4` | 下一交易日开盘价 | 下下交易日收盘价 | `(next2.close - next.open) / next.open` |

## 成功返回数据

```json
{
  "info": "ok",
  "code": 200,
  "data": {
    "name": "振幅因子",
    "info": "按振幅从低到高分位",
    "count": 5,
    "factor": [[0.01, 0.02]],
    "profit1": [
      {
        "source": [0.001, 0.002],
        "total_profit": 0.003,
        "total_net_value": 1.003002,
        "annualized_profit": 0.5475
      }
    ],
    "profit2": [],
    "profit3": [],
    "profit4": [],
    "datetime": ["2025-01-02", "2025-01-03"]
  }
}
```

示例省略了其他分位；实际 `factor` 和四类 `profit` 的外层长度都等于 `count`。

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `name` | string | 因子名称。 |
| `info` | string | 因子排序说明。 |
| `count` | integer | 分位数量。 |
| `datetime` | string[] | 实际迭代的交易日期。 |
| `factor` | number[][] | `[分位][日期]` 的组内平均因子值。 |
| `profit1..4` | Profit[] | 每种收益模式按分位保存的统计数据。 |

`Profit` 数据：

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `source` | number[] | 每个日期的组内平均单期收益。 |
| `total_profit` | number | 各期收益直接求和。 |
| `total_net_value` | number | 从 1 开始按 `1 + period_profit` 连乘。 |
| `annualized_profit` | number | `total_profit / period_count * 365`。 |

## 缓存

完整请求体序列化后计算 BLAKE3 哈希作为缓存键：

- 相同请求复用正在执行的任务。
- 已完成结果保存为 `cache/{hash}.json`。
- `base.id` 等所有请求字段都参与哈希。

## 错误响应

Content-Type 错误或 JSON 无法解析时通常返回 `415`：

```json
{
  "info": "ParseError: ...",
  "code": 415,
  "data": null
}
```

后台任务失败时返回 `400`：

```json
{
  "info": "获取数据失败",
  "code": 400,
  "data": null
}
```
