//! 公共数据模型与工具函数。

use anyhow::{bail, Context, Result};
use std::io::Read;
use std::path::Path;

/// 支持的工程格式。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Format {
    /// Scratch 3.0 (.sb3)
    Scratch,
    /// 腾讯扣叮 (.cdc)
    TencentCdc,
    /// 编程猫 Kitten 4 (.bcm4)
    Kitten,
}

impl Format {
    pub fn name(&self) -> &'static str {
        match self {
            Format::Scratch => "Scratch",
            Format::TencentCdc => "腾讯扣叮",
            Format::Kitten => "Kitten",
        }
    }
}

impl ProjectStats {
    /// 总量 = 积木 + 造型 + 音频。
    pub fn total(&self) -> u64 {
        self.blocks + self.costumes + self.sounds
    }
}

/// 分组统计（Scratch 专属：按角色/舞台分组）。
#[derive(Clone, Debug)]
pub struct GroupStat {
    pub name: String,
    pub blocks: u64,
    pub costumes: u64,
    pub sounds: u64,
}

/// 单个工程的统计结果。
#[derive(Clone, Debug)]
pub struct ProjectStats {
    pub path: String,
    pub format: Format,
    /// 角色/目标数量（含舞台）。
    pub actors: u64,
    /// 实体积木数（不含影子/输入积木）。
    pub blocks: u64,
    /// 影子积木（输入占位）数量，仅部分格式可识别。
    pub shadow_blocks: u64,
    pub costumes: u64,
    pub sounds: u64,
    /// 分组统计，仅 Scratch 有。
    pub groups: Vec<GroupStat>,
    /// 造型名清单，仅 CDC 有：(动画/角色名, [造型名...])。
    pub costume_groups: Vec<(String, Vec<String>)>,
}

/// 根据扩展名 / 文件头探测格式。
pub fn detect_format(path: &Path) -> Result<Format> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "sb3" => return Ok(Format::Scratch),
        "cdc" => return Ok(Format::TencentCdc),
        "bcm4" => return Ok(Format::Kitten),
        _ => {}
    }
    // 无已知扩展名时嗅探文件头。
    let mut head = [0u8; 4];
    let mut f = std::fs::File::open(path)
        .with_context(|| format!("无法打开文件: {}", path.display()))?;
    let n = std::io::Read::read(&mut f, &mut head)?;
    if n >= 2 && &head[..2] == b"PK" {
        // 两种 zip 工程：读 project.json 开头区分。
        let body = read_zip_entry(path, "project.json")?;
        let s = String::from_utf8_lossy(&body[..body.len().min(4096)]);
        if s.contains("\"targets\"") {
            Ok(Format::Scratch)
        } else if s.contains("\"scenes\"") || s.contains("\"blockType\"") {
            Ok(Format::TencentCdc)
        } else {
            bail!("无法识别的 zip 工程格式: {}", path.display())
        }
    } else if n >= 1 && head[0] == b'{' {
        // 纯 JSON 工程（Kitten bcm4 是纯 JSON）。
        let body = std::fs::read(path)?;
        let v: serde_json::Value =
            serde_json::from_slice(&body).context("文件以 { 开头但不是合法 JSON")?;
        if v.get("theatre").is_some() || v.get("work_type").is_some() {
            Ok(Format::Kitten)
        } else {
            bail!("无法识别的 JSON 工程格式: {}", path.display())
        }
    } else {
        bail!("无法识别的文件格式: {}（支持 .sb3 / .cdc / .bcm4）", path.display())
    }
}

/// 从 zip 工程中读取一个成员文件的完整字节。
pub fn read_zip_entry(path: &Path, name: &str) -> Result<Vec<u8>> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("无法打开文件: {}", path.display()))?;
    let mut zip = zip::ZipArchive::new(std::io::BufReader::new(file))
        .with_context(|| format!("不是有效的 zip 工程: {}", path.display()))?;
    let mut entry = zip
        .by_name(name)
        .with_context(|| format!("zip 内找不到 {name}: {}", path.display()))?;
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf)?;
    Ok(buf)
}

/// 数出 s 中出现的 `<block` 标签个数（用于扣叮 XML 积木统计）。
pub fn count_xml_tag(s: &str, tag: &str) -> u64 {
    let pat = format!("<{tag}");
    let mut n = 0u64;
    let mut rest = s;
    while let Some(pos) = rest.find(&pat) {
        let after = rest[pos + pat.len()..].chars().next();
        let ok = match after {
            None => false,
            Some(c) => c.is_whitespace() || c == '>' || c == '/',
        };
        if ok {
            n += 1;
        }
        rest = &rest[pos + pat.len()..];
    }
    n
}

/// 字符终端显示宽度（CJK 算 2 格）。
pub fn display_width(s: &str) -> usize {
    s.chars().map(char_width).sum()
}

fn char_width(c: char) -> usize {
    let u = c as u32;
    let wide = (0x1100..=0x115F).contains(&u)
        || (0x2E80..=0x303E).contains(&u)
        || (0x3041..=0x33FF).contains(&u)
        || (0x3400..=0x4DBF).contains(&u)
        || (0x4E00..=0x9FFF).contains(&u)
        || (0xA000..=0xA4CF).contains(&u)
        || (0xAC00..=0xD7A3).contains(&u)
        || (0xF900..=0xFAFF).contains(&u)
        || (0xFE30..=0xFE4F).contains(&u)
        || (0xFF00..=0xFF60).contains(&u)
        || (0xFFE0..=0xFFE6).contains(&u);
    if wide {
        2
    } else {
        1
    }
}
