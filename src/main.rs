use std::collections::HashMap;
use std::{env, io};

use chardet::detect;
use encoding::DecoderTrap;
use encoding::label::encoding_from_whatwg_label;
use rayon::prelude::*;
use std::fs::File;
use std::io::{BufReader, ErrorKind, Read};
use std::path::Path;
use std::time::Instant;
use walkdir::WalkDir;

mod model;

use model::{CliOptions, CodeFileData, ParserKind};

mod comment_parser;
use crate::comment_parser::{
    LuaState, ParseState, PythonState, classify_line_c_like, classify_line_css_like,
    classify_line_lua_like, classify_line_python_like, classify_line_xml_like,
};

const APP_NAME: &str = "cloc";
const APP_VERSION: &str = "1.0.0";

fn show_version() {
    println!("{APP_NAME}(rust) {APP_VERSION} @2026 by Loccy");
}

fn show_help() {
    println!(
        r#"用法:
  cloc [options] [path]

Arguments:
  path                扫描目录 (默认当前目录)

Options:
  -h, --help          显示帮助信息
  -V, --version       显示版本信息
  --no-parallel       禁用并行解析(rayon)
  --max-bytes <N>     跳过大文件，默认16M(16777216字节)
  --no-binary-skip    不跳过疑似二进制文件
  --exclude-dir <N>   排除目录， 默认排除目录(.git, target, node_modules)

示例:
  cloc .
  cloc --exclude-dir target --exclude-dir .git .
  cloc --no-parallel D:\\repo
  cloc --max-bytes 1048576 .
"#
    );
}

fn show_header() {
    println!("-------------------------------------------------------------------------------");
    println!(
        "{:<W$} {:>W$} {:>W$} {:>W$} {:>W$}",
        "Language",
        "files",
        "blank",
        "comment",
        "code",
        W = 15
    );
    println!("-------------------------------------------------------------------------------");
}

fn show_dash_line() {
    println!("-------------------------------------------------------------------------------");
}

/// Single source of truth for:
/// - which extensions are supported
/// - which parser to use
///
/// To add a new file type, add one entry here.
const PATTERNS: &[(&str, ParserKind)] = &[
    // C-like
    ("c", ParserKind::CLike),
    ("cpp", ParserKind::CLike),
    ("h", ParserKind::CLike),
    ("rs", ParserKind::CLike),
    ("java", ParserKind::CLike),
    ("go", ParserKind::CLike),
    ("swift", ParserKind::CLike),
    ("cs", ParserKind::CLike),
    ("m", ParserKind::CLike),
    ("mm", ParserKind::CLike),
    ("kt", ParserKind::CLike),
    ("js", ParserKind::CLike),
    ("ts", ParserKind::CLike),
    ("jsx", ParserKind::CLike),
    ("tsx", ParserKind::CLike),
    ("dart", ParserKind::CLike),
    // Python / Lua
    ("py", ParserKind::Python),
    ("lua", ParserKind::Lua),
    // Markup
    ("html", ParserKind::Xml),
    ("htm", ParserKind::Xml),
    ("xml", ParserKind::Xml),
    // Styles
    ("css", ParserKind::Css),
    ("scss", ParserKind::Css),
    ("less", ParserKind::Css),
];

fn parser_for_ext(ext: &str) -> Option<ParserKind> {
    // Linear scan is fine here; extensions list is tiny.
    PATTERNS.iter().find(|(e, _)| *e == ext).map(|(_, k)| *k)
}

fn parse_args() -> Result<CliOptions, String> {
    let mut opts = CliOptions::default();

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                show_help();
                std::process::exit(0);
            }
            "-V" | "--version" => {
                show_version();
                std::process::exit(0);
            }
            "--no-parallel" => {
                opts.parallel = false;
            }
            "--no-binary-skip" => {
                opts.binary_skip = false;
            }
            "--exclude-dir" => {
                let Some(v) = args.next() else {
                    return Err("--exclude-dir requires a value".to_string());
                };
                // allow user to add more excludes on top of defaults
                opts.exclude_dirs.push(v);
            }
            "--max-bytes" => {
                let Some(v) = args.next() else {
                    return Err("--max-bytes requires a value".to_string());
                };
                opts.max_bytes = v
                    .parse::<u64>()
                    .map_err(|_| format!("invalid --max-bytes value: {v}"))?;
            }
            _ => {
                if arg.starts_with('-') {
                    return Err(format!("unknown option: {arg}"));
                }
                // positional path (first one wins)
                opts.path = arg;
            }
        }
    }

    Ok(opts)
}

fn main() {
    let opts = match parse_args() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}\n");
            show_help();
            std::process::exit(2);
        }
    };

    let path = opts.path.as_str();

    // 用单调时钟计时，避免系统时间跳变导致误差
    let time_start = Instant::now();

    // 1) 串行扫描目录，只做轻量过滤（不读文件内容）
    let mut ignore_files: u64 = 0;
    let mut candidates: Vec<(String, String)> = Vec::new();

    // Build a lowercased exclude set for fast checks (case-insensitive on Windows).
    let exclude_dirs: Vec<String> = opts
        .exclude_dirs
        .iter()
        .map(|s| s.to_ascii_lowercase())
        .collect();

    for entry in WalkDir::new(path)
        .into_iter()
        .filter_entry(|e| {
            // Always keep root.
            if e.depth() == 0 {
                return true;
            }
            // Skip excluded directories.
            if e.file_type().is_dir() {
                if let Some(name) = e.file_name().to_str() {
                    let name_lc = name.to_ascii_lowercase();
                    return !exclude_dirs.iter().any(|x| x == &name_lc);
                }
            }
            true
        })
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }

        let Some(f_path) = entry.path().to_str() else {
            ignore_files += 1;
            continue;
        };

        let ext_opt = Path::new(f_path)
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .map(|s| s.to_ascii_lowercase());

        let Some(ext) = ext_opt else {
            ignore_files += 1;
            continue;
        };

        // Single source of truth: decide parser from extension.
        let Some(_kind) = parser_for_ext(ext.as_str()) else {
            ignore_files += 1;
            continue;
        };

        candidates.push((f_path.to_owned(), ext));
    }

    // 2) 解析文件：可并行/可串行
    let parsed: Vec<Option<CodeFileData>> = if opts.parallel {
        candidates
            .par_iter()
            .map(|(p, ext)| parse_file(p.as_str(), ext.as_str(), &opts))
            .collect()
    } else {
        candidates
            .iter()
            .map(|(p, ext)| parse_file(p.as_str(), ext.as_str(), &opts))
            .collect()
    };

    // 3) 合并结果
    let mut code_file_list: Vec<CodeFileData> = Vec::new();
    for item in parsed {
        match item {
            Some(cfd) => code_file_list.push(cfd),
            None => ignore_files += 1,
        }
    }

    let code_files = code_file_list.len() as u64;

    let mut map: HashMap<String, (u64, u64, u64, u64)> = HashMap::new();
    let mut sum: (u64, u64, u64, u64) = (0, 0, 0, 0);

    for cfi in &code_file_list {
        let key = cfi.patten();

        let v = map.entry(String::from(key)).or_insert((0, 0, 0, 0));
        v.0 += 1;
        v.1 += cfi.blank();
        v.2 += cfi.comment();
        v.3 += cfi.code();

        sum.0 += 1;
        sum.1 += cfi.blank();
        sum.2 += cfi.comment();
        sum.3 += cfi.code();
    }

    let time_used = time_start.elapsed().as_millis();

    println!();
    println!("Time used: {time_used} ms");
    println!("{:>10} code files", code_files);
    println!("{:>10} files ignored", ignore_files);
    println!();

    show_version();
    show_header();

    // Print in alphabetical order by language
    let mut rows: Vec<(&String, &(u64, u64, u64, u64))> = map.iter().collect();
    rows.sort_by(|(a, _), (b, _)| a.cmp(b));
    for (key, value) in rows {
        println!(
            "{:<W$} {:>W$} {:>W$} {:>W$} {:>W$}",
            key,
            value.0,
            value.1,
            value.2,
            value.3,
            W = 15
        );
    }

    show_dash_line();
    println!(
        "{:<W$} {:>W$} {:>W$} {:>W$} {:>W$}",
        "SUM",
        sum.0,
        sum.1,
        sum.2,
        sum.3,
        W = 15
    );
    show_dash_line();
}

fn parse_file(path: &str, ext: &str, opts: &CliOptions) -> Option<CodeFileData> {
    let kind = parser_for_ext(ext)?;

    // Read & parse file (respect CLI options)
    match kind {
        ParserKind::CLike => parse_code_file(path, ext, opts),
        ParserKind::Python => parse_python_file(path, ext, opts),
        ParserKind::Lua => parse_lua_file(path, ext, opts),
        ParserKind::Xml => parse_xml_file(path, ext, opts),
        ParserKind::Css => parse_css_file(path, ext, opts),
    }
}

// 使用//和/* */注释规则
fn parse_code_file(path: &str, ext: &str, opts: &CliOptions) -> Option<CodeFileData> {
    let mut cfd = CodeFileData::new(String::from(path), String::from(ext));
    let result = read_non_utf8_lines(path, opts.max_bytes, opts.binary_skip);
    if let Ok(content) = &result {
        cfd.set_lines(content.lines().count() as u64);

        let mut state = ParseState::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                cfd.add_blank();
                continue;
            }

            let (saw_code, saw_comment) = classify_line_c_like(line, &mut state);
            if saw_comment {
                cfd.add_comment();
            }
            if saw_code {
                cfd.add_code();
            }

            // In rare cases, a non-empty line might be neither code nor comment (shouldn't happen);
            // treat it as code to avoid losing counts.
            if !saw_code && !saw_comment {
                cfd.add_code();
            }
        }
        Some(cfd)
    } else {
        None
    }
}

// python 使用#和""" """注释规则
fn parse_python_file(path: &str, ext: &str, opts: &CliOptions) -> Option<CodeFileData> {
    let mut cfd = CodeFileData::new(String::from(path), String::from(ext));
    let result = read_non_utf8_lines(path, opts.max_bytes, opts.binary_skip);
    if let Ok(content) = &result {
        cfd.set_lines(content.lines().count() as u64);

        let mut state = PythonState::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                cfd.add_blank();
                continue;
            }

            let (saw_code, saw_comment) = classify_line_python_like(line, &mut state);
            if saw_comment {
                cfd.add_comment();
            }
            if saw_code {
                cfd.add_code();
            }
            if !saw_code && !saw_comment {
                cfd.add_code();
            }
        }
        Some(cfd)
    } else {
        None
    }
}

// lua 使用--和--[[ ]]注释规则
fn parse_lua_file(path: &str, ext: &str, opts: &CliOptions) -> Option<CodeFileData> {
    let mut cfd = CodeFileData::new(String::from(path), String::from(ext));
    let result = read_non_utf8_lines(path, opts.max_bytes, opts.binary_skip);
    if let Ok(content) = &result {
        cfd.set_lines(content.lines().count() as u64);

        let mut state = LuaState::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                cfd.add_blank();
                continue;
            }

            let (saw_code, saw_comment) = classify_line_lua_like(line, &mut state);
            if saw_comment {
                cfd.add_comment();
            }
            if saw_code {
                cfd.add_code();
            }
            if !saw_code && !saw_comment {
                cfd.add_code();
            }
        }
        Some(cfd)
    } else {
        None
    }
}

// xml、html 使用<!-- -->注释规则
fn parse_xml_file(path: &str, ext: &str, opts: &CliOptions) -> Option<CodeFileData> {
    let mut cfd = CodeFileData::new(String::from(path), String::from(ext));
    let result = read_non_utf8_lines(path, opts.max_bytes, opts.binary_skip);
    if let Ok(content) = &result {
        cfd.set_lines(content.lines().count() as u64);

        let mut state = ParseState::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                cfd.add_blank();
                continue;
            }

            let (saw_code, saw_comment) = classify_line_xml_like(line, &mut state);
            if saw_comment {
                cfd.add_comment();
            }
            if saw_code {
                cfd.add_code();
            }
            if !saw_code && !saw_comment {
                cfd.add_code();
            }
        }
        Some(cfd)
    } else {
        None
    }
}

// css, 使用/* */注释规则
fn parse_css_file(path: &str, ext: &str, opts: &CliOptions) -> Option<CodeFileData> {
    let mut cfd = CodeFileData::new(String::from(path), String::from(ext));
    let result = read_non_utf8_lines(path, opts.max_bytes, opts.binary_skip);
    if let Ok(content) = &result {
        cfd.set_lines(content.lines().count() as u64);

        let mut state = ParseState::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                cfd.add_blank();
                continue;
            }

            let (saw_code, saw_comment) = classify_line_css_like(line, &mut state);
            if saw_comment {
                cfd.add_comment();
            }
            if saw_code {
                cfd.add_code();
            }
            if !saw_code && !saw_comment {
                cfd.add_code();
            }
        }
        Some(cfd)
    } else {
        None
    }
}

fn read_non_utf8_lines(path: &str, max_bytes: u64, binary_skip: bool) -> io::Result<String> {
    let file = File::open(path)?;

    if let Ok(meta) = file.metadata() {
        if meta.len() > max_bytes {
            return Err(io::Error::new(ErrorKind::InvalidData, "文件过大，已跳过"));
        }
    }

    let mut reader = BufReader::new(file);
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;

    if binary_skip {
        // Heuristic: skip likely-binary files early (NUL byte is a strong signal).
        if buf.iter().take(8192).any(|&b| b == 0) {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                "疑似二进制文件，已跳过",
            ));
        }
    }

    if let Ok(s) = std::str::from_utf8(&buf) {
        return Ok(s.to_owned());
    }

    let charset = detect(&buf);
    let enc_label = charset.0;
    if let Some(enc) = encoding_from_whatwg_label(enc_label.as_str()) {
        match enc.decode(&buf, DecoderTrap::Replace) {
            Ok(content) => return Ok(content),
            Err(_) => eprintln!("解码失败: {}", path),
        }
    }

    Err(io::Error::new(ErrorKind::InvalidData, "无法识别的编码"))
}
