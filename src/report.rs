//! 统计报告输出：终端表格（CJK 对齐）、JSON、CSV。

use crate::model::{display_width, ProjectStats};

/// 汇总表（多文件对比）。
pub fn print_summary_table(all: &[ProjectStats]) {
    if all.is_empty() {
        return;
    }
    let headers = ["文件", "格式", "角色", "积木", "造型", "音频", "总量"];
    let rows: Vec<Vec<String>> = all
        .iter()
        .map(|s| {
            let file = std::path::Path::new(&s.path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&s.path)
                .to_string();
            vec![
                file,
                s.format.name().to_string(),
                s.actors.to_string(),
                s.blocks.to_string(),
                s.costumes.to_string(),
                s.sounds.to_string(),
                s.total().to_string(),
            ]
        })
        .collect();
    print_table(&headers, &rows);
}

/// 单文件详情：分组统计仅 Scratch 有，其余格式显示整体说明。
pub fn print_detail(stats: &ProjectStats) {
    println!();
    println!(
        "══ {} [{}] ══",
        stats.path,
        stats.format.name()
    );
    println!(
        "角色/目标: {}   积木: {}   造型: {}   音频: {}   总量: {}",
        stats.actors,
        stats.blocks,
        stats.costumes,
        stats.sounds,
        stats.total()
    );
    if stats.shadow_blocks > 0 {
        println!("（另含影子/输入积木 {} 个，未计入积木总量）", stats.shadow_blocks);
    }
    // 造型名清单（CDC 按动画/角色分组；Scratch 按角色/舞台）。
    if !stats.costume_groups.is_empty() {
        println!();
        println!("造型名（按角色/动画分组）:");
        let owner_width = stats
            .costume_groups
            .iter()
            .map(|(o, _)| display_width(o))
            .max()
            .unwrap_or(0);
        for (owner, names) in &stats.costume_groups {
            if names.is_empty() {
                println!("  {owner:<ow$}  （无）", ow = owner_width);
                continue;
            }
            // 每行放尽可能多的造型名，超宽换行并对齐到首列。
            let prefix_len = owner_width + 4;
            let mut line = String::new();
            for name in names {
                let sep = if line.is_empty() { "" } else { "、 " };
                if prefix_len + display_width(&line) + display_width(sep) + display_width(name)
                    > 96
                    && !line.is_empty()
                {
                    println!("  {owner:<ow$}  {line}", ow = owner_width);
                    line.clear();
                }
                if !line.is_empty() {
                    line.push_str(sep);
                }
                line.push_str(name);
            }
            println!("  {owner:<ow$}  {line}", ow = owner_width);
        }
    }
    if !stats.groups.is_empty() {
        println!();
        let headers = ["分组(角色/舞台)", "积木", "造型", "音频"];
        let rows: Vec<Vec<String>> = stats
            .groups
            .iter()
            .map(|g| {
                vec![
                    g.name.clone(),
                    g.blocks.to_string(),
                    g.costumes.to_string(),
                    g.sounds.to_string(),
                ]
            })
            .collect();
        print_table(&headers, &rows);
    }
}

/// 输出 JSON。
pub fn to_json(all: &[ProjectStats]) -> String {
    let items: Vec<serde_json::Value> = all
        .iter()
        .map(|s| {
            let mut item = serde_json::json!({
                "file": s.path,
                "format": s.format.name(),
                "actors": s.actors,
                "blocks": s.blocks,
                "shadow_blocks": s.shadow_blocks,
                "costumes": s.costumes,
                "sounds": s.sounds,
                "total": s.total(),
            });
            if !s.groups.is_empty() {
                item["groups"] = serde_json::Value::Array(
                    s.groups
                        .iter()
                        .map(|g| {
                            serde_json::json!({
                                "name": g.name,
                                "blocks": g.blocks,
                                "costumes": g.costumes,
                                "sounds": g.sounds,
                            })
                        })
                        .collect(),
                );
            }
            item
        })
        .collect();
    serde_json::to_string_pretty(&items).unwrap_or_else(|_| "[]".to_string())
}

/// 输出 CSV（UTF-8，首行表头；分组行跟随在每个工程行后）。
pub fn to_csv(all: &[ProjectStats]) -> String {
    let mut out = String::from("file,format,group,actors,blocks,shadow_blocks,costumes,sounds,total\n");
    for s in all {
        let file = csv_escape(&s.path);
        let fmt = s.format.name();
        if s.groups.is_empty() {
            out.push_str(&format!(
                "{file},{fmt},,{},{},{},{},{},{}\n",
                s.actors, s.blocks, s.shadow_blocks, s.costumes, s.sounds, s.total()
            ));
        } else {
            for g in &s.groups {
                out.push_str(&format!(
                    "{file},{fmt},{},{},{},{},{},{},{}\n",
                    csv_escape(&g.name),
                    s.actors,
                    g.blocks,
                    s.shadow_blocks,
                    g.costumes,
                    g.sounds,
                    g.blocks + g.costumes + g.sounds
                ));
            }
        }
    }
    out
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// 用空格对齐打印表格（按显示宽度，CJK 算 2 格）。
fn print_table(headers: &[&str], rows: &[Vec<String>]) {
    let ncols = headers.len();
    let mut widths = vec![0usize; ncols];
    for (i, h) in headers.iter().enumerate() {
        widths[i] = display_width(h);
    }
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < ncols {
                widths[i] = widths[i].max(display_width(cell));
            }
        }
    }
    let line = |cells: Vec<&str>| {
        let mut out = String::new();
        for (i, c) in cells.iter().enumerate() {
            if i > 0 {
                out.push_str("  ");
            }
            let pad = widths[i].saturating_sub(display_width(c));
            // 最后一列左对齐其余也左对齐，数字列由调用方决定；这里统一左对齐并在右侧补空格。
            out.push_str(c);
            out.push_str(&" ".repeat(pad));
        }
        println!("{}", out.trim_end());
    };

    line(headers.to_vec());
    // 分隔线。
    let sep: Vec<String> = widths.iter().map(|w| "─".repeat(*w)).collect();
    println!("{}", sep.join("  "));
    for row in rows {
        let refs: Vec<&str> = row.iter().map(|s| s.as_str()).collect();
        line(refs);
    }
}
