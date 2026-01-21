mod model;

use crate::model::FileInfo;
use model::CodeFileData;
use std::collections::HashMap;
/// calculates the code file lines
use std::{env, io};

use chardet::detect;
use encoding::label::encoding_from_whatwg_label;
use encoding::{DecoderTrap, Encoding};
use std::fs::File;
use std::io::{BufReader, ErrorKind, Read};

fn show_version() {
    println!("cloc 1.0.0 @2026 LOCCY");
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
    "lua", "cs", "xml", "kt","jsx","tsx","scss","less","dart","m","mm","vue"
];

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        show_help();
        return;
    }
    let path = &args[1];
    println!("dir={}", path);

    let mut code_files = 0;
    let mut ignore_files = 0;

    let file_list: Vec<FileInfo> = read_dir(path);

    let mut code_file_list: Vec<CodeFileData> = Vec::new();

    for fi in &file_list {
        if fi.is_code_file() {
            code_files += 1;
            let code_file_info = parse_file(fi.path(), fi.extension());
            code_file_list.push(code_file_info);
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

fn read_dir(path: &str) -> Vec<FileInfo> {
    let mut file_list: Vec<FileInfo> = Vec::new();

    let entries = std::fs::read_dir(path).unwrap();

    for entry in entries {
        if let Ok(dir_enter) = entry {
            let path = dir_enter.path();
            let path_str = path.to_str().unwrap();
            if path.is_dir() {
                let list = read_dir(path_str);
                for fi in list {
                    file_list.push(fi);
                }
            } else {
                let ext = path.extension().and_then(std::ffi::OsStr::to_str);
                if let Some(ext) = ext {
                    let is_code_file = EXTENSIONS.contains(&ext);
                    let fi = FileInfo::new(String::from(path_str), String::from(ext), is_code_file);
                    file_list.push(fi);
                }
            }
        }
    }

    file_list
}

fn parse_file(path: &str, ext: &str) -> CodeFileData {
    let mut fi = CodeFileData::new(String::from(path), String::from(ext));

    // let content = std::fs::read_to_string(path).unwrap();
    let result = read_non_utf8_lines(path);
    match result {
        Ok(content) => {
            let lines: Vec<&str> = content.lines().collect();
            fi.set_lines(lines.len() as u64);

            let mut is_comment_wrap = false;
            for line in lines {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    fi.add_blank();
                } else {
                    if trimmed.starts_with("//") {
                        fi.add_comment();
                    } else if trimmed.starts_with("/*") && trimmed.ends_with("*/") {
                        fi.add_comment();
                    } else if trimmed.starts_with("/*") {
                        fi.add_comment();
                        is_comment_wrap = true;
                    } else if is_comment_wrap {
                        fi.add_comment();
                        if trimmed.ends_with("*/") {
                            is_comment_wrap = false;
                        }
                    } else {
                        fi.add_code();
                    }
                }
            }
        }
        Err(_) => {}
    }



    fi
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
     Err(io::Error::new(ErrorKind::NotFound, "无法识别的编码") )
}
