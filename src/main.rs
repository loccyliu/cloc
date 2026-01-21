mod model;

use model::CodeFileData;
use std::collections::HashMap;
use std::{env, io};

use chardet::detect;
use encoding::label::encoding_from_whatwg_label;
use encoding::{DecoderTrap, Encoding};
use std::fs::File;
use std::io::{BufReader, ErrorKind, Read};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

fn show_version() {
    println!("cloc(rust) 1.0.0 @2026 by Loccy");
}
fn show_help() {
    println!("\nUsage: cloc <path>");
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

fn show_footer() {
    println!("-------------------------------------------------------------------------------");
}

const EXTENSIONS: &[&str] = &[
    "rs", "js", "ts", "py", "java", "c", "cpp", "h", "html", "css", "go", "rb", "php", "swift",
    "lua", "cs", "xml", "kt", "jsx", "tsx", "scss", "less", "dart", "m", "mm", "vue",
];

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        show_help();
        return;
    }
    let path = &args[1];
    println!("dir={}", path);
    let time_start = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis(); // 获取毫秒

    let mut code_files = 0;
    let mut ignore_files = 0;

    let file_list: Vec<String> = read_dir(path);

    let mut code_file_list: Vec<CodeFileData> = Vec::new();

    for f_path in &file_list {
        // 判断文件是否是代码文件
        let ext = Path::new(f_path)
            .extension()
            .and_then(std::ffi::OsStr::to_str);
        // 根据不同的扩展名，判断是哪种代码文件，不同的扩展名可能对应不同的编程语言，对应的注释规则也不同

        let code_file_info = parse_file(f_path, ext.unwrap());
        if let Some(cfi) = code_file_info {
            code_file_list.push(cfi);
            code_files += 1;
        } else {
            ignore_files += 1;
        }
    }

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
    let time_end = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis(); // 获取毫秒
    let time_used = time_end - time_start;

    println!();
    println!("Time used: {time_used} ms");
    println!("{:>10} code files", code_files);
    println!("{:>10} files ignored", ignore_files);

    show_version();
    show_header();
    for (key, value) in map.iter() {
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
    show_footer();
    println!(
        "{:<W$} {:>W$} {:>W$} {:>W$} {:>W$}",
        "SUM",
        sum.0,
        sum.1,
        sum.2,
        sum.3,
        W = 15
    );
    show_footer();
}

fn read_dir(path: &str) -> Vec<String> {
    let mut file_list: Vec<String> = Vec::new();

    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            if let Some(path_str) = entry.path().to_str() {
                file_list.push(String::from(path_str));
            }
        }
    }
    file_list
}

fn parse_file(path: &str, ext: &str) -> Option<CodeFileData> {
    match ext {
        "c" | "cpp" | "h" | "rs" | "java" | "go" | "swift" | "cs" | "m" | "mm" | "kt" | "js"
        | "ts" | "jsx" | "tsx" | "dart" => parse_code_file(path, ext),
        "py" => parse_python_file(path, ext),
        "lua" => parse_lua_file(path, ext),
        "html" | "htm" | "xml" => parse_xml_file(path, ext),
        "css" | "scss" | "less" => parse_css_file(path, ext),
        _ => None,
    }
}

// 使用//和/* */注释规则
fn parse_code_file(path: &str, ext: &str) -> Option<CodeFileData> {
    let mut cfd = CodeFileData::new(String::from(path), String::from(ext));
    let result = read_non_utf8_lines(path);
    if let Ok(content) = &result {
        let lines: Vec<&str> = content.lines().collect();
        cfd.set_lines(lines.len() as u64);

        let mut is_comment_wrap = false;
        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                cfd.add_blank();
            } else {
                if trimmed.starts_with("//") && !is_comment_wrap {
                    cfd.add_comment();
                } else if trimmed.starts_with("/*") && trimmed.ends_with("*/") {
                    cfd.add_comment();
                } else if trimmed.starts_with("/*") {
                    cfd.add_comment();
                    is_comment_wrap = true;
                } else if is_comment_wrap {
                    cfd.add_comment();
                    if trimmed.ends_with("*/") {
                        is_comment_wrap = false;
                    }
                } else {
                    cfd.add_code();
                }
            }
        }
        Some(cfd)
    } else {
        None
    }
}

// python 使用#和""" """注释规则
fn parse_python_file(path: &str, ext: &str) -> Option<CodeFileData> {
    let mut cfd = CodeFileData::new(String::from(path), String::from(ext));
    let result = read_non_utf8_lines(path);
    if let Ok(content) = &result {
        let lines: Vec<&str> = content.lines().collect();
        cfd.set_lines(lines.len() as u64);

        let mut is_comment_wrap = false;
        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                cfd.add_blank();
            } else {
                if trimmed.starts_with("#") && !is_comment_wrap {
                    cfd.add_comment();
                } else if trimmed.starts_with("\"\"\"") && trimmed.ends_with("\"\"\"") {
                    cfd.add_comment();
                } else if trimmed.starts_with("\"\"\"") {
                    cfd.add_comment();
                    is_comment_wrap = true;
                } else if is_comment_wrap {
                    cfd.add_comment();
                    if trimmed.ends_with("\"\"\"") {
                        is_comment_wrap = false;
                    }
                } else {
                    cfd.add_code();
                }
            }
        }
        Some(cfd)
    } else {
        None
    }
}

// lua 使用--和--[[ ]]注释规则
fn parse_lua_file(path: &str, ext: &str) -> Option<CodeFileData> {
    let mut cfd = CodeFileData::new(String::from(path), String::from(ext));
    let result = read_non_utf8_lines(path);
    if let Ok(content) = &result {
        let lines: Vec<&str> = content.lines().collect();
        cfd.set_lines(lines.len() as u64);

        let mut is_comment_wrap = false;
        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                cfd.add_blank();
            } else {
                if trimmed.starts_with("--") && !is_comment_wrap {
                    cfd.add_comment();
                } else if trimmed.starts_with("--[[") && trimmed.ends_with("]]") {
                    cfd.add_comment();
                } else if trimmed.starts_with("--[[") {
                    cfd.add_comment();
                    is_comment_wrap = true;
                } else if is_comment_wrap {
                    cfd.add_comment();
                    if trimmed.ends_with("]]") {
                        is_comment_wrap = false;
                    }
                } else {
                    cfd.add_code();
                }
            }
        }
        Some(cfd)
    } else {
        None
    }
}

// xml、html 使用<!-- -->注释规则
fn parse_xml_file(path: &str, ext: &str) -> Option<CodeFileData> {
    let mut cfd = CodeFileData::new(String::from(path), String::from(ext));
    let result = read_non_utf8_lines(path);
    if let Ok(content) = &result {
        let lines: Vec<&str> = content.lines().collect();
        cfd.set_lines(lines.len() as u64);

        let mut is_comment_wrap = false;
        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                cfd.add_blank();
            } else {
                if trimmed.starts_with("<!--") && trimmed.ends_with("-->") {
                    cfd.add_comment();
                } else if trimmed.starts_with("<!--") {
                    cfd.add_comment();
                    is_comment_wrap = true;
                } else if is_comment_wrap {
                    cfd.add_comment();
                    if trimmed.ends_with("-->") {
                        is_comment_wrap = false;
                    }
                } else {
                    cfd.add_code();
                }
            }
        }
        Some(cfd)
    } else {
        None
    }
}

// css, 使用/* */注释规则
fn parse_css_file(path: &str, ext: &str) -> Option<CodeFileData> {
    let mut cfd = CodeFileData::new(String::from(path), String::from(ext));
    let result = read_non_utf8_lines(path);
    if let Ok(content) = &result {
        let lines: Vec<&str> = content.lines().collect();
        cfd.set_lines(lines.len() as u64);

        let mut is_comment_wrap = false;
        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                cfd.add_blank();
            } else {
                if trimmed.starts_with("/*") && trimmed.ends_with("*/") {
                    cfd.add_comment();
                } else if trimmed.starts_with("/*") {
                    cfd.add_comment();
                    is_comment_wrap = true;
                } else if is_comment_wrap {
                    cfd.add_comment();
                    if trimmed.ends_with("*/") {
                        is_comment_wrap = false;
                    }
                } else {
                    cfd.add_code();
                }
            }
        }
        Some(cfd)
    } else {
        None
    }
}

fn read_non_utf8_lines(path: &str) -> io::Result<String> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;
    // 自动检测编码
    let charset = detect(&buf);
    let enc_label = charset.0; // 返回类似 "GB18030", "SHIFT_JIS" 
    // println!("检测到编码: {}", enc_label);
    if let Some(enc) = encoding_from_whatwg_label(enc_label.as_str()) {
        match enc.decode(&buf, DecoderTrap::Replace) {
            Ok(content) => {
                return Ok(content);
            }
            Err(_) => {
                eprintln!("解码失败");
            }
        }
    }
    Err(io::Error::new(ErrorKind::NotFound, "无法识别的编码"))
}
