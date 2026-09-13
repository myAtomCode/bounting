# Bounting

快速统计 **Scratch / 腾讯扣叮 / Kitten** 工程项目的积木数量、造型数量、音频数量与总量（分组统计仅限 Scratch）。

## 支持格式

| 扩展名 | 平台 | 说明 |
|---|---|---|
| `.sb3` | Scratch 3.0 | zip 工程，按角色/舞台分组统计 |
| `.cdc` | 腾讯扣叮 | zip 工程，积木为 XML，造型来自 animates |
| `.bcm4` | 编程猫 Kitten 4 | 纯 JSON 工程 |

扩展名未知时会按文件头自动嗅探。

## 安装

```sh
cargo install bounting
```

或从源码构建：

```sh
git clone https://github.com/myAtomCode/bounting
cd bounting
cargo build --release
# 产物在 target/release/bounting
```

## 用法

```sh
bounting [选项] <工程文件...>
```

| 选项 | 说明 |
|---|---|
| `--json` | 输出 JSON（含 Scratch 分组） |
| `--csv` | 输出 CSV（每个分组一行） |
| `--quiet` / `-q` | 只打印汇总表，不打印详情 |

## 示例

```sh
$ bounting 作品.sb3 全面战争模拟器.cdc 新的作品.bcm4
文件                 格式      角色  积木  造型  音频  总量
──────────────────  ────────  ────  ────  ────  ────  ────
作品 (1).sb3        Scratch   2     0     2     0     2
全面战争模拟器.cdc  腾讯扣叮  72    8022  209   0     8231
新的作品 (1).bcm4   Kitten    1     7     6     0     13
```

Scratch 工程还会输出每个角色/舞台的分组明细表；影子积木（输入占位）单独标注，不重复计入总量。

## 统计口径

- **积木**：实体积木数（Scratch/Kitten 排除影子积木；扣叮统计 `<block` 标签）。
- **造型**：Scratch `costumes[]`、扣叮 `animates[].images[]`、Kitten `styles[]`。
- **音频**：Scratch `sounds[]`、扣叮场景 `audios[]` 加 zip 内音频文件兜底扫描、Kitten 全局与节点内音频。
- **总量** = 积木 + 造型 + 音频。

## License

MIT
