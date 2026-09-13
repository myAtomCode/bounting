//! 编程猫 Kitten 4 (.bcm4) 统计。
//!
//! bcm4 是纯 JSON：
//! - `theatre.actors{}`：角色字典，`styles[]` 是造型 id 列表，
//!   `block_data_json.blocks{}` 是积木字典；
//! - `theatre.scenes{}`：舞台/场景，同样有 `styles[]` 与
//!   `block_data_json.blocks{}`；
//! - 顶层 `audio{}`/`audio_order[]` 是全局音频，
//!   角色/场景对象内也可能有 `audios` 数组（旧版结构）。

use crate::model::{Format, ProjectStats};
use anyhow::Result;
use serde_json::Value;
use std::path::Path;

/// 统计一个 .bcm4 工程。
pub fn analyze(path: &Path) -> Result<ProjectStats> {
    let body = std::fs::read(path)?;
    let root: Value =
        serde_json::from_slice(&body).map_err(|e| anyhow::anyhow!("bcm4 JSON 解析失败: {e}"))?;

    let mut stats = ProjectStats {
        path: path.display().to_string(),
        format: Format::Kitten,
        actors: 0,
        blocks: 0,
        shadow_blocks: 0,
        costumes: 0,
        sounds: 0,
        groups: Vec::new(),
        costume_groups: Vec::new(),
    };

    let theatre = root
        .get("theatre")
        .ok_or_else(|| anyhow::anyhow!("bcm4 缺少 theatre 字段"))?;

    // 角色。
    if let Some(actors) = theatre.get("actors").and_then(|a| a.as_object()) {
        for (_id, actor) in actors {
            stats.actors += 1;
            count_node(actor, &mut stats);
        }
    }

    // 场景/舞台（scene 也是积木和造型的宿主）。
    if let Some(scenes) = theatre.get("scenes").and_then(|s| s.as_object()) {
        for (_id, scene) in scenes {
            count_node(scene, &mut stats);
        }
    }

    // 全局音频：audio 字典或 audio_order 数组。
    match root.get("audio") {
        Some(Value::Object(o)) => stats.sounds += o.len() as u64,
        Some(Value::Array(a)) => stats.sounds += a.len() as u64,
        _ => {}
    }
    if stats.sounds == 0 {
        if let Some(order) = root.get("audio_order").and_then(|a| a.as_array()) {
            stats.sounds += order.len() as u64;
        }
    }

    Ok(stats)
}

/// 统计一个角色/场景节点：积木、影子积木、造型、节点内音频。
fn count_node(node: &Value, stats: &mut ProjectStats) {
    // 积木字典。
    if let Some(blocks) = node
        .get("block_data_json")
        .and_then(|b| b.get("blocks"))
        .and_then(|b| b.as_object())
    {
        for (_id, b) in blocks {
            let is_shadow = b
                .get("is_shadow")
                .and_then(|s| s.as_bool())
                .unwrap_or(false);
            if is_shadow {
                stats.shadow_blocks += 1;
            } else {
                stats.blocks += 1;
            }
        }
    }

    // 造型。
    if let Some(styles) = node.get("styles").and_then(|s| s.as_array()) {
        stats.costumes += styles.len() as u64;
    }

    // 节点内音频（部分版本结构）。
    if let Some(audios) = node.get("audios").and_then(|a| a.as_array()) {
        stats.sounds += audios.len() as u64;
    }
}
