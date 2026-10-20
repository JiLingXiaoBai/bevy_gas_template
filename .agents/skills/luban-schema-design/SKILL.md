---
name: luban-schema-design
description: Designs Luban schema (beans, enums, polymorphism, collections, groups). Use when modeling config structures, inheritance, nested beans, or choosing Excel vs XML schema.
---

## bevy_gas_template project context

Before using this skill in this repository, read the [AI skills project conventions](../../../.docs/02-luban-toolchain.md#ai-skills).
Use those conventions for paths, MCP calls, export commands, and Rust/Bevy integration; the upstream examples below are generic.
Configuration code belongs to `bevy_gas_template::config`; the `bevy_gas` dependency contains the GAS runtime.
Use this game's `tools/luban/` and local `.cache/`; published binary data belongs in `assets/config/`.
This repository maintains GAS table registrations, enums, and explicit inline beans in `config/defines/gas.xml`, with table row fields read from Excel data headers.
The existing `gas.AbilityAdditionalCost` bean has `resource: int` (Rust `i32`) and `amount: long`, without a separate cost-row ID; abilities own an inline `additional_costs` list. Do not recreate the former cost table or its row references.
Allow the full i32 resource range, including zero and negatives; the game provider decides whether a resource exists. Keep amount in `1..=4294967295`.
Preserve list order and duplicate entries; check each ability's total per resource ID without merging authored entries. Pass the i32 ID directly to GAS `AdditionalCost`; providers match the same integer without resource registration or a name-pool mapping. The compiler's `resources` map is removed; the preparation `totals` map only checks per-ability quantity overflow and never reorders or merges the output list. Keep name-pool registration for tags and attributes.
See the [configuration guide](../../../.docs/03-gas-configuration.md#技能内联额外消耗) for the format and migration from string names; regenerate the full package after changing the schema.
Add other beans to that XML only when needed; do not duplicate row beans inferred from the data headers.
`builtin.xml` contains Luban built-ins, and the Excel Schema files mentioned below are generic alternatives.


# Luban: Schema 设计

## 原则

- 程序维护 Schema；策划填 Data。
- 复杂 GamePlay（技能/行为树）优先 OOP 继承/多态，而不是塞字符串。
- 客户端敏感字段用 `s` group，不要泄漏到 `c`。

## 选型

| 需求 | 建议 |
|------|------|
| 扁平行表 | Excel + `read_schema_from_file` |
| 多模块/复用 bean | XML `Defines/*.xml` 或 `__beans__` |
| 多态配置 | 抽象 bean + 子类；Excel 填类型名/别名 |
| 一对多嵌套 | `list,Bean` / 多行 / sep 紧凑格式 |

## 类型要点

- 容器：`list,T` / `map,K,V`；**元素不可写 `list,int?`**
- 可空：`T?`；多态非空必须给具体子类
- 引用：`int#ref=module.TbX`
- 字段名建议 `snake_case`，生成时按语言转风格

## 检查清单

1. 主键与 `mode`（map/list/one）是否匹配
2. group 是否覆盖 client/server 需求
3. 多态子类是否都已定义且可区分
4. 用 `-c schema-json` 或 MCP `GetSchema` 复核结构

## 参考

- 类型系统、XML Schema、多态、分组 targets
