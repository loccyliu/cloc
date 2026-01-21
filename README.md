# cloc (Rust)

一个轻量的 `cloc` 风格代码行数统计工具（Rust 实现）。

- 递归扫描目录
- 按语言（基于文件扩展名）汇总统计：`blank / comment / code`
- 支持常见注释规则（`// /* */ # """ -- <!-- -->` 等）
- 默认跳过常见大目录：`.git / target / node_modules`
- 支持并行解析（rayon），适合大仓库

> 说明：本项目按“行”统计，属于 cloc 风格的近似统计，不是完整语法解析器。

## 安装 / 构建

本项目是 Rust Cargo 工程。

```bash
cargo build --release
```

生成的可执行文件在：

- Windows：`target\\release\\cloc.exe`
- Linux/macOS：`target/release/cloc`

## 基本用法

```bash
# 默认扫描当前目录
cloc

# 扫描指定目录
cloc <path>
```

输出示例：

- `Language`：语言/分类（当前实现里就是扩展名）
- `files`：该语言文件数
- `blank/comment/code`：空行/注释行/代码行

## CLI 参数

```text
-h, --help          显示帮助信息
-V, --version       显示版本信息
--no-parallel       禁用并行解析(rayon)
--max-bytes <N>     跳过大文件，默认16M(16777216字节)
--no-binary-skip    不跳过疑似二进制文件
--exclude-dir <N>   排除目录（可重复），默认排除: .git, target, node_modules
```

### 示例

```bash
# 排除更多目录（可重复）
cloc --exclude-dir dist --exclude-dir .idea .

# 禁用并行（小项目/调试时可能更方便）
cloc --no-parallel .

# 只统计较小文件
cloc --max-bytes 1048576 .
```

## 支持的文件类型

通过 `src/main.rs` 中的 `PATTERNS` 维护扩展名与解析器映射（**单一来源**）。目前支持：

- C-like：`c, cpp, h, rs, java, go, swift, cs, m, mm, kt, js, ts, jsx, tsx, dart`
- Python：`py`
- Lua：`lua`
- Markup：`html, htm, xml`
- Styles：`css, scss, less`

> 想增加新的类型：优先在 `PATTERNS` 增加扩展名映射；如果注释规则不同，再新增对应的解析分支。

## 注释/代码行判定规则（概览）

本项目使用轻量状态机做“按行分类”，主要目标是避免一些常见误判：

- C-like：支持 `//` 行注释与 `/* ... */` 块注释（可跨行），并尽量忽略字符串内的注释符号。
- Python：支持 `#` 行注释与 triple-quote（`"""` / `'''`）块注释，并支持 `end""" x = 1` 这种同一行结束后还有代码。
- Lua：支持 `--` 行注释与 `--[[ ... ]]` 块注释。
- XML/HTML：支持 `<!-- ... -->`。
- CSS：支持 `/* ... */`。

相关实现与测试：

- 解析器：`src/comment_parser.rs`
- 测试：`tests/comment_parser_tests.rs`

## 性能与注意事项

- 默认启用并行解析（rayon），大仓库更快；小仓库可能并行开销略高，可用 `--no-parallel`。
- 默认跳过疑似二进制文件（前 8KB 出现 NUL 字节）；如有需要可用 `--no-binary-skip`。
- 默认跳过大文件（`--max-bytes`，默认 16MiB）。

## 开发

```bash
cargo test
cargo run -- --help
```

## License

MIT License. See `LICENSE`.
