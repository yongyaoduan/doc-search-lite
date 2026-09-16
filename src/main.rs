mod parse;

use anyhow::{Context, Result, ensure};
use clap::{Parser, Subcommand};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::json;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    time::{Duration, UNIX_EPOCH},
};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Override the local SQLite index location.
    #[arg(long, global = true)]
    db: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Import a file or recursively sync a directory. Unchanged files are skipped.
    Index {
        path: PathBuf,
        /// Reparse even when size and modification time are unchanged.
        #[arg(long)]
        force: bool,
    },
    /// Search locally; returns bounded JSON excerpts with source locations.
    Search {
        query: String,
        #[arg(long, default_value_t = 5, value_parser = clap::value_parser!(u32).range(1..=20))]
        limit: u32,
    },
    /// Show index size and document/chunk counts.
    Status,
    /// Remove an indexed file or directory from the index, leaving source files intact.
    Remove { path: PathBuf },
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let db_path = match cli.db {
        Some(path) => path,
        None => std::env::current_exe()?
            .parent()
            .context("executable directory unavailable")?
            .join("data/index.db"),
    };
    if let Some(parent) = db_path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let mut db = Connection::open(&db_path)?;
    db.busy_timeout(Duration::from_secs(5))?;
    db.execute_batch(
        "PRAGMA cache_size=-4096;
        CREATE TABLE IF NOT EXISTS documents(path TEXT PRIMARY KEY, stamp TEXT NOT NULL);
        CREATE VIRTUAL TABLE IF NOT EXISTS chunks USING fts5(
            path UNINDEXED, location UNINDEXED, text UNINDEXED, body, name);",
    )?;
    match cli.command {
        Command::Index { path, force } => index(&mut db, &path, force)?,
        Command::Search { query, limit } => search(&db, &query, limit)?,
        Command::Status => println!(
            "{}",
            json!({
                "database": db_path,
                "bytes": db_path.metadata()?.len(),
                "documents": db.query_row("SELECT count(*) FROM documents", [], |r| r.get::<_, i64>(0))?,
                "chunks": db.query_row("SELECT count(*) FROM chunks", [], |r| r.get::<_, i64>(0))?,
            })
        ),
        Command::Remove { path } => {
            let path = absolute(&path)?;
            let paths = indexed_paths(&db)?;
            let tx = db.transaction()?;
            let mut removed = 0;
            for source in paths {
                if Path::new(&source).starts_with(&path) {
                    forget(&tx, &source)?;
                    removed += 1;
                }
            }
            tx.commit()?;
            println!("{}", json!({"removed": removed}));
        }
    }
    Ok(())
}

// Strip Windows verbatim prefixes so sources can be pasted into ordinary tools.
fn absolute(path: &Path) -> Result<PathBuf> {
    let result = if path.exists() {
        path.canonicalize()?
    } else {
        std::path::absolute(path)?
    };
    #[cfg(windows)]
    {
        let text = result.to_string_lossy();
        if let Some(tail) = text.strip_prefix(r"\\?\UNC\") {
            return Ok(PathBuf::from(format!(r"\\{tail}")));
        }
        if let Some(tail) = text.strip_prefix(r"\\?\") {
            return Ok(PathBuf::from(tail));
        }
    }
    Ok(result)
}

fn indexed_paths(db: &Connection) -> Result<Vec<String>> {
    Ok(db
        .prepare("SELECT path FROM documents")?
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?)
}

fn forget(db: &Connection, path: &str) -> Result<()> {
    db.execute("DELETE FROM chunks WHERE path=?1", [path])?;
    db.execute("DELETE FROM documents WHERE path=?1", [path])?;
    Ok(())
}

fn index(db: &mut Connection, input: &Path, force: bool) -> Result<()> {
    ensure!(input.exists(), "path does not exist: {}", input.display());
    ensure!(
        !input.is_file() || parse::supported(input),
        "unsupported file type; use DOCX, XLSX, PPTX, text PDF, TXT, MD, or CSV"
    );
    let root = absolute(input)?;
    let mut imported = 0;
    let mut skipped = 0;
    let mut errors = Vec::new();
    // No symlink traversal: avoid loops and accidentally indexing another tree.
    for entry in walkdir::WalkDir::new(&root).follow_links(false) {
        let entry = entry?;
        if !entry.file_type().is_file() || !parse::supported(entry.path()) {
            continue;
        }
        let path = absolute(entry.path())?;
        let source = path.to_str().context("file path must be valid Unicode")?;
        let meta = path.metadata()?;
        let stamp = format!(
            "{}:{}",
            meta.len(),
            meta.modified()?.duration_since(UNIX_EPOCH)?.as_nanos()
        );
        let previous: Option<String> = db
            .query_row("SELECT stamp FROM documents WHERE path=?1", [source], |r| {
                r.get(0)
            })
            .optional()?;
        if !force && previous.as_deref() == Some(&stamp) {
            skipped += 1;
            continue;
        }
        eprintln!("Indexing {}", path.display());
        let result = std::panic::catch_unwind(|| parse::extract(&path))
            .unwrap_or_else(|_| Err(anyhow::anyhow!("document parser panicked")));
        let sections = match result {
            Ok(sections) => sections,
            Err(error) => {
                let tx = db.transaction()?;
                forget(&tx, source)?;
                tx.commit()?;
                errors.push(json!({"path": source, "error": format!("{error:#}")}));
                continue;
            }
        };
        let tx = db.transaction()?;
        forget(&tx, source)?;
        let name = tokens(
            &path.file_name().unwrap_or_default().to_string_lossy(),
            true,
        )
        .join(" ");
        for section in sections {
            let chars: Vec<char> = section.text.chars().collect();
            for start in (0..chars.len()).step_by(780) {
                let end = (start + 900).min(chars.len());
                let text: String = chars[start..end].iter().collect();
                if !text.trim().is_empty() {
                    let location = format!("{}, chars {}–{}", section.location, start + 1, end);
                    tx.execute(
                        "INSERT INTO chunks(path,location,text,body,name) VALUES(?1,?2,?3,?4,?5)",
                        params![source, location, text, tokens(&text, true).join(" "), name],
                    )?;
                }
                if end == chars.len() {
                    break;
                }
            }
        }
        tx.execute(
            "INSERT INTO documents(path,stamp) VALUES(?1,?2)",
            params![source, stamp],
        )?;
        tx.commit()?;
        imported += 1;
    }
    let mut removed = 0;
    let tx = db.transaction()?;
    for source in indexed_paths(&tx)? {
        let path = Path::new(&source);
        if path.starts_with(&root) && !path.try_exists()? {
            forget(&tx, &source)?;
            removed += 1;
        }
    }
    tx.commit()?;
    println!(
        "{}",
        json!({"imported": imported, "skipped": skipped, "removed": removed, "errors": errors})
    );
    ensure!(
        errors.is_empty(),
        "some files could not be indexed; see JSON errors"
    );
    Ok(())
}

fn search(db: &Connection, query: &str, limit: u32) -> Result<()> {
    ensure!(query.chars().count() <= 500, "query exceeds 500 characters");
    let terms: BTreeSet<_> = tokens(query, false).into_iter().collect();
    ensure!(
        !terms.is_empty(),
        "query must contain letters, numbers, or CJK characters"
    );
    ensure!(
        terms.len() <= 64,
        "query exceeds 64 search terms; use a shorter query"
    );
    let expression = terms
        .iter()
        .map(|t| format!("\"{t}\""))
        .collect::<Vec<_>>()
        .join(" OR ");
    let mut statement = db.prepare(
        "SELECT path,location,text,bm25(chunks,0,0,0,1,2) AS score
        FROM chunks WHERE chunks MATCH ?1 ORDER BY score LIMIT ?2",
    )?;
    let rows = statement
        .query_map(params![expression, limit], |row| {
            Ok(json!({
                "path": row.get::<_, String>(0)?,
                "location": row.get::<_, String>(1)?,
                "text": row.get::<_, String>(2)?,
                "score": -row.get::<_, f64>(3)?,
            }))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    println!("{}", json!({"query": query, "results": rows}));
    Ok(())
}

fn cjk(c: char) -> bool {
    matches!(c as u32, 0x3400..=0x9fff | 0xf900..=0xfaff | 0x20000..=0x323af | 0x3040..=0x30ff | 0xac00..=0xd7af)
}

// CJK bigrams avoid a bundled dictionary/model. Keep unigrams in the index
// for single-character queries; multi-character queries use only bigrams.
fn tokens(text: &str, index: bool) -> Vec<String> {
    let chars: Vec<char> = text.chars().flat_map(char::to_lowercase).collect();
    let mut result = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let start = i;
        if cjk(chars[i]) {
            while i < chars.len() && cjk(chars[i]) {
                i += 1;
            }
            if index || i - start == 1 {
                result.extend(chars[start..i].iter().map(ToString::to_string));
            }
            result.extend(chars[start..i].windows(2).map(|w| w.iter().collect()));
        } else if chars[i].is_alphanumeric() {
            while i < chars.len() && chars[i].is_alphanumeric() && !cjk(chars[i]) {
                i += 1;
            }
            result.push(chars[start..i].iter().collect());
        } else {
            i += 1;
        }
    }
    result
}
