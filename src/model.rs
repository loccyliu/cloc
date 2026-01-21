//! 数据结构体

pub enum ExtType {
    JS,
    Other,
}

pub struct FileInfo {
    path: String,
    extension: String,
    is_code_file: bool,
}

impl FileInfo {
    pub fn new(path: String, extension: String, is_code_file: bool) -> FileInfo {
        FileInfo {
            path,
            extension,
            is_code_file,
        }
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn extension(&self) -> &str {
        &self.extension
    }

    pub fn is_code_file(&self) -> bool {
        self.is_code_file
    }
}

pub struct CodeFileData {
    path: String,
    patten: String,
    lines: u64,
    blank: u64,
    comment: u64,
    code: u64,
}

impl CodeFileData {
    pub fn new(path: String, patten: String) -> CodeFileData {
        CodeFileData {
            path,
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

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn patten(&self) -> &str {
        &self.patten
    }

    pub fn lines(&self) -> u64 {
        self.lines
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
