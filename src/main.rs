//! Bounting —— 快速统计 Scratch(.sb3) / 腾讯扣叮(.cdc) / Kitten(.bcm4)
//! 工程的积木数、造型数、音频数与总量（分组统计仅限 Scratch）。
//!
//! 用法:
//!   bounting <文件...>            汇总表 + 每个文件的详情（含 Scratch 分组）
//!   bounting --json <文件...>     输出 JSON
//!   bounting --csv  <文件...>     输出 CSV
//!   bounting --quiet <文件...>    只输出汇总表

mod cdc;
mod kitten;
mod model;
mod report;
mod scratch;

use anyhow::{bail, Result};
use model::{detect_format, Format, ProjectStats};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        print_usage();
        std::process::exit(if args.is_empty() { 1 } else { 0 });
    }

    let mut mode = Output::Table;
    let mut files: Vec<PathBuf> = Vec::new();
    for a in &args {
        match a.as_str() {
            "--json" => mode = Output::Json,
            "--csv" => mode = Output::Csv,
            "--quiet" | "-q" => mode = Output::Quiet,
            _ => files.push(PathBuf::from(a)),
        }
    }
    if files.is_empty() {
        print_usage();
        std::process::exit(1);
    }

    if let Err(e) = run(&files, mode) {
        eprintln!("bounting: {e:#}");
        std::process::exit(1);
    }
}

enum Output {
    Table,
    Json,
    Csv,
    Quiet,
}

fn run(files: &[PathBuf], mode: Output) -> Result<()> {
    let mut all: Vec<ProjectStats> = Vec::new();
    let mut failed = 0usize;

    for f in files {
        if !f.is_file() {
            eprintln!("bounting: 跳过（不是文件）: {}", f.display());
            failed += 1;
            continue;
        }
        match analyze_file(f) {
            Ok(s) => all.push(s),
            Err(e) => {
                eprintln!("bounting: 分析 {} 失败: {e:#}", f.display());
                failed += 1;
            }
        }
    }

    if all.is_empty() {
        bail!("没有任何文件分析成功");
    }

    match mode {
        Output::Json => println!("{}", report::to_json(&all)),
        Output::Csv => print!("{}", report::to_csv(&all)),
        Output::Quiet => report::print_summary_table(&all),
        Output::Table => {
            report::print_summary_table(&all);
            for s in &all {
                report::print_detail(s);
            }
        }
    }

    if failed > 0 {
        bail!("{failed} 个文件分析失败（其余结果已输出）");
    }
    Ok(())
}

fn analyze_file(path: &std::path::Path) -> Result<ProjectStats> {
    let format = detect_format(path)?;
    match format {
        Format::Scratch => scratch::analyze(path),
        Format::TencentCdc => cdc::analyze(path),
        Format::Kitten => kitten::analyze(path),
    }
}

fn print_usage() {
    println!(
        "Bounting —— 统计 Scratch / 腾讯扣叮 / Kitten 工程\n\
         \n\
         用法: bounting [选项] <工程文件...>\n\
         \n\
         支持格式: .sb3 (Scratch 3)  .cdc (腾讯扣叮)  .bcm4 (Kitten 4)\n\
         \n\
         选项:\n\
           --json   输出 JSON（含 Scratch 分组）\n\
           --csv    输出 CSV（含 Scratch 分组）\n\
           --quiet  只打印汇总表，不打印详情\n\
         \n\
         示例:\n\
           bounting 作品.sb3\n\
           bounting --json 全面战争模拟器.cdc 新的作品.bcm4"
    );
}
