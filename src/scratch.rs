//! Scratch 3.0 (.sb3) 统计。
//!
//! sb3 是 zip 包，`project.json` 里 `targets` 数组每个元素是一个
//! 角色（sprite）或舞台（stage），积木在 `blocks` 字典中
//! （key 为积木 id，value 为积木对象或字符串压缩引用）。

use crate::model::{read_zip_entry, Format, GroupStat, ProjectStats};
use anyhow::Result;
use serde_json::Value;
use std::path::Path;

/// 统计一个 .sb3 工程。
pub fn analyze(path: &Path) -> Result<ProjectStats> {
    let body = read_zip_entry(path, "project.json")?;
    let root: Value =
        serde_json::from_slice(&body).map_err(|e| anyhow::anyhow!("project.json 解析失败: {e}"))?;
    let targets = root
        .get("targets")
        .and_then(|t| t.as_array())
        .ok_or_else(|| anyhow::anyhow!("project.json 缺少 targets 数组"))?;

    let mut stats = ProjectStats {
        path: path.display().to_string(),
        format: Format::Scratch,
        actors: targets.len() as u64,
        blocks: 0,
        shadow_blocks: 0,
        costumes: 0,
        sounds: 0,
        groups: Vec::new(),
        costume_groups: Vec::new(),
    };

    for target in targets {
        let name = target
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("<未命名>")
            .to_string();

        let mut group = GroupStat {
            name,
            blocks: 0,
            costumes: 0,
            sounds: 0,
        };

        // 积木：值为对象的是实体积木；值为字符串的是压缩字段引用，不计数。
        if let Some(blocks) = target.get("blocks").and_then(|b| b.as_object()) {
            for (_id, b) in blocks {
                match b {
                    Value::Object(obj) => {
                        let shadow = obj.get("shadow").and_then(|s| s.as_bool()).unwrap_or(false);
                        // 顶层脚本（topLevel）也算积木本身，无需特殊处理。
                        if shadow {
                            stats.shadow_blocks += 1;
                        } else {
                            group.blocks += 1;
                        }
                    }
                    _ => {} // 压缩引用，跳过
                }
            }
        }

        group.costumes = target
            .get("costumes")
            .and_then(|c| c.as_array())
            .map(|a| a.len() as u64)
            .unwrap_or(0);
        // 收集造型名，供详情展示。
        if let Some(costumes) = target.get("costumes").and_then(|c| c.as_array()) {
            let names: Vec<String> = costumes
                .iter()
                .filter_map(|c| c.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
                .collect();
            stats.costume_groups.push((group.name.clone(), names));
        }
        group.sounds = target
            .get("sounds")
            .and_then(|s| s.as_array())
            .map(|a| a.len() as u64)
            .unwrap_or(0);

        stats.blocks += group.blocks;
        stats.costumes += group.costumes;
        stats.sounds += group.sounds;
        stats.groups.push(group);
    }

    Ok(stats)
}
