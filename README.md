# doc-search-lite

[![Build, test, release](https://github.com/yongyaoduan/doc-search-lite/actions/workflows/ci.yml/badge.svg)](https://github.com/yongyaoduan/doc-search-lite/actions/workflows/ci.yml)

[English](#english) | [简体中文](#简体中文)

## English

**A small, portable, fully offline document search skill for Chinese and English.**

Extract document text, build a local BM25 index, and return relevant excerpts with source locations to your agent.
One native executable, with no Python, Node, model downloads, database server, or background process.

### Download and use

Download the ZIP for your platform from [Releases](https://github.com/yongyaoduan/doc-search-lite/releases/latest):

| Platform | Filename suffix |
| --- | --- |
| Windows x64 | `windows-x64.zip` |
| Mac, Apple Silicon | `macos-arm64.zip` |
| Mac, Intel | `macos-x64.zip` |

Extract and run. No installation, administrator privileges, or network connection is needed to use the tool. Each ZIP includes the executable, skill, and dependency licenses. CI enforces a **5 MiB** limit on the ZIP size.

**As a Codex skill:** place the extracted `doc-search-lite` folder in `~/.agents/skills/` (`%USERPROFILE%\.agents\skills\` on Windows), restart your agent, and ask:

> Use $doc-search-lite to index my documents folder, then find the submarine cable acceptance requirements and cite the sources.

For other agents that support SKILL.md, place the same folder in their skills directory; discovery depends on the agent.
This tool does not connect to the network. Whether the final answer uses a cloud service depends on your agent and model.

**As a command-line tool:** extract to any writable directory and run from that directory:

```sh
# macOS; in Windows PowerShell, replace ./doc-search-lite with .\doc-search-lite.exe
./doc-search-lite index "/path/to/documents"
./doc-search-lite search "cable acceptance" --limit 5
./doc-search-lite search "光缆 验收"
./doc-search-lite status
```

There is no graphical interface; double-clicking the executable does not open a chat window. Use the skill for everyday interaction.
Release files are not Windows code-signed or Apple-notarized, so your system may ask you to allow the program on first use.

### Supported content

| Format | Extracted content and source locations |
| --- | --- |
| Word `.docx` | Body text and tables; character positions within the document |
| Excel `.xlsx` | Worksheet cells and saved formula results; sheet names and row numbers |
| PowerPoint `.pptx` | Slide text and tables; slide numbers in presentation order |
| PDF `.pdf` | Extractable text; page numbers |
| `.txt` / `.md` / `.csv` | UTF-8 text; character positions |

**Not included:** semantic search, OCR, image/chart understanding, scanned-document recognition, or legacy binary `.doc` / `.xls` / `.ppt` files.
Save legacy Office files as `.docx` / `.xlsx` / `.pptx` first.
The first version does not extract Word headers, footers, or comments, or PowerPoint speaker notes. Excel formulas are not recalculated, and not all display formatting is preserved.
PDF text extraction depends on font encoding. Encrypted files and files without extractable text are reported as errors.

### Indexing and search

- Chinese uses character unigrams/bigrams; English is split into words. SQLite FTS5 ranks results with BM25. No tokenization dictionary is needed.
- Searches return 5 excerpts by default, each at most 900 characters. `--limit` accepts 1–20.
- Results are JSON with `path`, `location`, `text`, and `score`. Search matches keywords and does not understand synonyms.
- The index is stored in `data/index.db` beside the executable, with no background service. After moving source documents, index their new paths and remove the old paths from the index.
- Running `index` again skips files whose size and modification time are unchanged, updates modified files, and removes index entries for files deleted from that directory.
- Use `index <directory> --force` to rebuild. If parsing fails, old excerpts for that file are removed; successfully imported files remain searchable.
- Files are processed sequentially, with a 100 MiB limit per file. Large or unusual documents may use more memory; this is not a sandbox with a fixed memory cap.

```sh
# Select a separate index; use the same --db for subsequent searches
./doc-search-lite --db "/path/to/project.db" index "/path/to/documents"
./doc-search-lite --db "/path/to/project.db" search "procurement amount"

# Remove index entries only; source documents are left intact
./doc-search-lite remove "/path/to/documents"
```

To upgrade, replace the executable and skill with the new ZIP contents and keep `data/`. Delete the entire tool folder to remove the tool and its default index.

### Development and releases

Only development machines need Rust and a C compiler toolchain. Python 3.11+ is used only for packaging and smoke tests; it is not shipped with the application.

```sh
cargo fmt --check
cargo clippy --locked -- -D warnings
cargo build --release --locked
python packaging/package.py
```

The code consists of two Rust files: command/index logic and document extraction. One smoke test checks Chinese/English retrieval from actual document containers, source locations, incremental updates, deletion, error handling, and output length.

GitHub Actions builds on Windows, Apple Silicon Mac, and Intel Mac, extracts the actual release ZIPs, and tests the executables with their PATH cleared. This verifies that no additional runtime is needed; it does not replace manual verification across all Windows/macOS versions and system security prompts.

Pushing to `main` or opening a PR triggers CI. After updating the version, push a `vX.Y.Z` tag matching Cargo.toml. Once all platforms pass, CI automatically publishes a release, three ZIPs, and SHA-256 checksums.

Dependencies are pinned in Cargo.lock. After updating dependencies, run `python packaging/licenses.py` to refresh the bundled licenses.

MIT License.

---

## 简体中文

**免安装、中英文、完全离线的轻量文档检索 Skill。**

提取文档文字，在本地建立 BM25 索引，只把相关片段和来源交给 Agent。
一个原生程序，无 Python、Node、模型下载、数据库服务或后台进程。

### 下载与使用

在 [Releases](https://github.com/yongyaoduan/doc-search-lite/releases/latest) 下载对应的 ZIP：

| 系统 | 文件名后缀 |
| --- | --- |
| Windows x64 | `windows-x64.zip` |
| Mac，Apple Silicon | `macos-arm64.zip` |
| Mac，Intel | `macos-x64.zip` |

解压即用，不需要安装、管理员权限或网络。每个 ZIP 包含完整程序、Skill 和依赖许可，CI 将压缩包上限限制为 **5 MiB**。

**作为 Codex Skill：**把解压出的整个 `doc-search-lite` 文件夹放到 `~/.agents/skills/`（Windows 为 `%USERPROFILE%\.agents\skills\`），重新打开 Agent，然后说：

> 使用 $doc-search-lite，索引我的资料目录，然后查找海底光缆的验收要求，注明来源。

其他支持 SKILL.md 的 Agent 可将同一文件夹放入其 Skills 目录；具体发现方式由 Agent 决定。
本工具本身不联网；最终回答是否经过云端，取决于你使用的 Agent 和模型。

**作为命令行工具：**可解压到任意可写目录，在该目录运行：

```sh
# macOS；Windows PowerShell 将 ./doc-search-lite 换成 .\doc-search-lite.exe
./doc-search-lite index "/path/to/documents"
./doc-search-lite search "光缆 验收" --limit 5
./doc-search-lite search "cable acceptance"
./doc-search-lite status
```

程序没有图形界面；双击可执行文件不会打开聊天窗口。日常操作交给 Skill 即可。
发布文件尚未做 Windows 签名或 Apple 公证，系统首次运行可能要求允许该程序。

### 支持的内容

| 格式 | 提取内容与来源 |
| --- | --- |
| Word `.docx` | 正文、表格；文档内字符位置 |
| Excel `.xlsx` | 工作表单元格、已保存的公式计算结果；表名与行号 |
| PowerPoint `.pptx` | 幻灯片文字、表格；实际幻灯片顺序 |
| PDF `.pdf` | 可提取的文本；页码 |
| `.txt` / `.md` / `.csv` | UTF-8 文本；字符位置 |

**不包含**语义检索、OCR、图片/图表理解、扫描件识别，或旧版二进制 `.doc` / `.xls` / `.ppt`。
旧版 Office 文件请先另存为对应的 `.docx` / `.xlsx` / `.pptx`。
Word 页眉页脚、批注，PPT 备注不在首版提取范围；Excel 不重新计算公式或保留全部显示格式。
PDF 的可提取程度受其字体编码影响；加密文件和无可提取文字的文件会报告错误。

### 索引与检索

- 中文用单字/双字切分，英文按词切分，SQLite FTS5 BM25 排序；无需分词词典。
- 默认返回 5 个片段，每段最多 900 字符；`--limit` 范围为 1–20。
- 返回 JSON：`path`、`location`、`text`、`score`。查询按关键词匹配，不理解同义词。
- 索引位于程序旁的 `data/index.db`，无后台服务。移动原始文档后，重新索引新路径并移除旧路径。
- 再次 `index` 会跳过大小和修改时间没变的文件，更新修改过的文件，清理该目录下已删除的文件。
- 用 `index <目录> --force` 强制重建。解析失败不会保留该文件的旧片段；其他成功导入的文件仍可检索。
- 单文件上限 100 MiB，按文件顺序处理。超大或异常文档可能需要更多内存；这不是固定内存沙箱。

```sh
# 单独指定索引；后续查询也传入同一个 --db
./doc-search-lite --db "/path/to/project.db" index "/path/to/documents"
./doc-search-lite --db "/path/to/project.db" search "采购 金额"

# 只删除索引记录，原始文档保持不变
./doc-search-lite remove "/path/to/documents"
```

升级时，用新 ZIP 覆盖程序和 Skill，保留 `data/`。删除整个工具文件夹即可移除工具及默认索引。

### 开发与发布

仅开发机需要 Rust 和 C 编译工具链；Python 3.11+ 只用于打包和运行冒烟测试，不进入交付包。

```sh
cargo fmt --check
cargo clippy --locked -- -D warnings
cargo build --release --locked
python packaging/package.py
```

代码分为命令/索引逻辑和文档提取两个 Rust 文件。一个冒烟测试验证真实文件容器的中英文检索、来源位置、增量更新、删除、错误处理和输出长度。

GitHub Actions 在 Windows、Apple Silicon Mac、Intel Mac 上构建，解压实际交付 ZIP，清空被测程序的 PATH 后执行测试。它验证无需额外运行时，但不代替 Windows/macOS 所有版本和安全提示的人工验证。

推送 `main` 或提交 PR 会触发 CI；版本更新后推送匹配 Cargo.toml 的 `vX.Y.Z` 标签，全部平台通过后自动发布 Release、三个 ZIP 和 SHA-256 校验文件。

依赖由 Cargo.lock 固定；更新依赖后运行 `python packaging/licenses.py` 更新随包许可。

MIT License.
