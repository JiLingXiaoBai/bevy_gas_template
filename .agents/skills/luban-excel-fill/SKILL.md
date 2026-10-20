---
name: luban-excel-fill
description: "Explains and fills Luban Excel config tables by header conventions. Use when designers fill sheets, ##var/##type/##group rows, nested cells, or polymorphism columns."
---

## bevy_gas_template project context

Before using this skill in this repository, read the [AI skills project conventions](../../../.docs/02-luban-toolchain.md#ai-skills).
Use those conventions for paths, MCP calls, export commands, and Rust/Bevy integration; the upstream examples below are generic.
Configuration code belongs to `bevy_gas_template::config`; the `bevy_gas` dependency contains the GAS runtime.
Use this game's `tools/luban/` and local `.cache/`; published binary data belongs in `assets/config/`.
Ability `additional_costs` is an inline typed list of `gas.AbilityAdditionalCost` (`resource: int`, `amount: long`), without separate cost-row references. Preserve authored order and duplicates; an empty list means no extra cost.
The current field type is `(list#sep=;),gas.AbilityAdditionalCost`; the bean uses comma-separated integer `resource,amount` fields, such as `5001,1;5002,3`. Resource IDs allow the full i32 range, including zero and negatives; amount must be in `1..=4294967295`.
Use the existing headers and the [configuration guide](../../../.docs/03-gas-configuration.md#技能内联额外消耗) for examples. Map old resource names to game-defined numeric IDs and regenerate; do not recreate the removed cost workbook.


# Luban: Excel 填表

## 最小约定

| 行 | 含义 |
|----|------|
| `##var` / `##` | 字段名 |
| `##type` | 类型 |
| `##group` | `c`/`s`/`e`；空=跟随默认 |
| `##` 注释行 | 中文说明，不进逻辑 |
| `#` 开头列名 | 注释列，不导出 |

- A1 必须以 `##` 开头，否则整张 sheet 忽略。
- 空字符串填 `""`（按项目约定）。
- 枚举可填名字或 alias。

## 分组提醒

- `c`：客户端可见
- `s`：仅服务器
- 填错会导致缺字段或敏感数据下发

## 复杂结构

- 嵌套 / 列表：遵循程序给定的分列或多行样表，不要自创分隔符
- 多态：按样表填具体类型名；拿不准先问程序或查 schema-json

## Agent 行为

1. 改数值前确认列的 `##type` 与 group。
2. 不擅自改 `##type` / 主键列语义。
3. 生成失败时保留行列信息，用 `luban-generate-debug` 排查。

## 参考

- 策划概念、表头、复杂结构、Excel 嵌套
