//! 腾讯扣叮 (.cdc) 统计。
//!
//! cdc 是 zip 包：
//! - `project.json` 的 `scenes[].sprites[]` 是角色/舞台，
//!   每个角色的 `blocks` 字段是一段积木 XML（`<block .../>` 嵌套）；
//! - 顶层 `animates[]` 是造型动画，每个动画的 `images[]` 是造型帧，
//!   `blobUrl` 指向 zip 内的图片文件；
//! - `scenes[].audios[]` 是场景音频（样本里可能为空，音频也可能
//!   以 zip 内音频文件形式存在，额外按文件头兜底统计）。

use crate::model::{count_xml_tag, read_zip_entry, Format, ProjectStats};
use anyhow::Result;
use serde_json::Value;
use std::path::Path;

/// 统计一个 .cdc 工程。
pub fn analyze(path: &Path) -> Result<ProjectStats> {
    let body = read_zip_entry(path, "project.json")?;
    let root: Value =
        serde_json::from_slice(&body).map_err(|e| anyhow::anyhow!("project.json 解析失败: {e}"))?;

    let mut stats = ProjectStats {
        path: path.display().to_string(),
        format: Format::TencentCdc,
        actors: 0,
        blocks: 0,
        shadow_blocks: 0,
        costumes: 0,
        sounds: 0,
        groups: Vec::new(),
        costume_groups: Vec::new(),
    };

    // 角色 / 积木 / 场景音频。
    if let Some(scenes) = root.get("scenes").and_then(|s| s.as_array()) {
        for scene in scenes {
            if let Some(sprites) = scene.get("sprites").and_then(|s| s.as_array()) {
                stats.actors += sprites.len() as u64;
                for sprite in sprites {
                    if let Some(xml) = sprite.get("blocks").and_then(|b| b.as_str()) {
                        // `<block`：实体积木；`<shadow`：输入占位（近似影子积木）。
                        stats.blocks += count_xml_tag(xml, "block");
                        stats.shadow_blocks += count_xml_tag(xml, "shadow");
                    }
                }
            }
            if let Some(audios) = scene.get("audios").and_then(|a| a.as_array()) {
                stats.sounds += audios.len() as u64;
            }
        }
    }

    // 造型：animates[].images[]，同时收集造型名。
    if let Some(animates) = root.get("animates").and_then(|a| a.as_array()) {
        for anim in animates {
            if let Some(images) = anim.get("images").and_then(|i| i.as_array()) {
                stats.costumes += images.len() as u64;
                let names: Vec<String> = images
                    .iter()
                    .filter_map(|img| {
                        img.get("name")
                            .and_then(|n| n.as_str())
                            .map(|s| s.to_string())
                    })
                    .collect();
                let owner = anim
                    .get("animName")
                    .and_then(|n| n.as_str())
                    .unwrap_or("<未命名>")
                    .to_string();
                stats.costume_groups.push((owner, names));
            }
        }
    }

    // 音频兜底：扫 zip 成员文件头（WAV / MP3 / OGG）。
    stats.sounds += count_zip_audio_files(path);

    Ok(stats)
}

/// 通过文件魔数统计 zip 内的音频文件个数。
fn count_zip_audio_files(path: &Path) -> u64 {
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return 0,
    };
    let mut zip = match zip::ZipArchive::new(std::io::BufReader::new(file)) {
        Ok(z) => z,
        Err(_) => return 0,
    };
    let mut n = 0u64;
    let names: Vec<String> = zip.file_names().map(|s| s.to_string()).collect();
    for name in names {
        let is_audio_ext = {
            let lower = name.to_ascii_lowercase();
            lower.ends_with(".mp3")
                || lower.ends_with(".wav")
                || lower.ends_with(".ogg")
                || lower.ends_with(".m4a")
                || lower.ends_with(".aac")
                || lower.ends_with(".flac")
        };
        if is_audio_ext {
            n += 1;
            continue;
        }
        // 扣叮素材是无扩展名哈希名，读前 16 字节判魔数。
        let mut buf = [0u8; 16];
        let len = match zip.by_name(&name) {
            Ok(mut e) => std::io::Read::read(&mut e, &mut buf).unwrap_or(0),
            Err(_) => 0,
        };
        if is_audio_magic(&buf[..len]) {
            n += 1;
        }
    }
    n
}

fn is_audio_magic(b: &[u8]) -> bool {
    // "RIFF"...."WAVE"
    if b.len() >= 12 && &b[0..4] == b"RIFF" && &b[8..12] == b"WAVE" {
        return true;
    }
    // ID3 或 MP3 帧同步 0xFF Ex/Fx
    if b.len() >= 3 && &b[0..3] == b"ID3" {
        return true;
    }
    if b.len() >= 2 && b[0] == 0xFF && (b[1] & 0xE0) == 0xE0 {
        return true;
    }
    // OggS
    if b.len() >= 4 && &b[0..4] == b"OggS" {
        return true;
    }
    false
}
