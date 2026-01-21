//! 数据结构体


#[derive(Clone, Copy)]
pub enum ParserKind {
    CLike,
    Python,
    Lua,
    Xml,
    Css,
}


#[derive(Debug, Clone)]
pub(crate) struct CliOptions {
    pub(crate) path: String,
    pub(crate) parallel: bool,
    pub(crate) max_bytes: u64,
    pub(crate) binary_skip: bool,
    pub(crate) exclude_dirs: Vec<String>,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self {
            path: ".".to_string(),
            parallel: true,
            max_bytes: 16 * 1024 * 1024, // 16MiB
            binary_skip: true,
            exclude_dirs: vec![
                ".git".to_string(),
                "target".to_string(),
                "node_modules".to_string(),
            ],
        }
    }
}

pub struct CodeFileData {
    patten: String,
    lines: u64,
    blank: u64,
    comment: u64,
    code: u64,
}

impl CodeFileData {
    pub fn new(path: String, patten: String) -> CodeFileData {
        CodeFileData {
            patten,
            lines: 0,
            blank: 0,
            comment: 0,
            code: 0,
        }
    }
    pub fn add_blank(&mut self) {
        self.blank += 1;
    }

    pub fn add_comment(&mut self) {
        self.comment += 1;
    }

    pub fn add_code(&mut self) {
        self.code += 1;
    }

    pub fn set_lines(&mut self, lines: u64) {
        self.lines = lines;
    }

    pub fn patten(&self) -> &str {
        &self.patten
    }

    pub fn blank(&self) -> u64 {
        self.blank
    }

    pub fn comment(&self) -> u64 {
        self.comment
    }

    pub fn code(&self) -> u64 {
        self.code
    }
}
