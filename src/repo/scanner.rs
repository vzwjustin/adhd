use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::util::errors::Result;

/// Results of scanning a repo directory tree.
#[derive(Debug, Clone)]
pub struct RepoScan {
    pub root: PathBuf,
    pub file_count: usize,
    pub languages: Vec<LanguageInfo>,
    pub build_files: Vec<String>,
    pub test_patterns: Vec<String>,
    pub config_files: Vec<String>,
    pub todo_fixme_hack: Vec<TodoItem>,
    pub directory_clusters: Vec<DirCluster>,
    pub likely_build_cmd: Option<String>,
    pub likely_test_cmd: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LanguageInfo {
    pub name: String,
    pub extensions: Vec<String>,
    pub file_count: usize,
}

#[derive(Debug, Clone)]
pub struct TodoItem {
    pub path: String,
    pub line_number: usize,
    pub kind: TodoKind,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TodoKind {
    Todo,
    Fixme,
    Hack,
    Xxx,
}

impl TodoKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Todo => "TODO",
            Self::Fixme => "FIXME",
            Self::Hack => "HACK",
            Self::Xxx => "XXX",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DirCluster {
    pub path: String,
    pub file_count: usize,
    pub dominant_language: Option<String>,
}

/// Known ignore patterns — directories to skip during scanning.
const IGNORE_DIRS: &[&str] = &[
    "node_modules",
    "target",
    ".git",
    "dist",
    "__pycache__",
    ".venv",
    "vendor",
    ".next",
    "build",
    ".cache",
    "coverage",
    ".tox",
    "venv",
    "env",
    ".mypy_cache",
    ".pytest_cache",
    ".cargo",
];

/// Scan a repo for structure, languages, build files, TODOs.
pub fn scan_repo(root: &Path, max_depth: usize) -> Result<RepoScan> {
    let mut ext_counts: HashMap<String, usize> = HashMap::new();
    let mut build_files = Vec::new();
    let mut test_patterns = Vec::new();
    let mut config_files = Vec::new();
    let mut todo_items = Vec::new();
    let mut dir_file_counts: HashMap<String, usize> = HashMap::new();
    let mut file_count: usize = 0;

    walk_dir(
        root,
        root,
        0,
        max_depth,
        &mut ext_counts,
        &mut build_files,
        &mut test_patterns,
        &mut config_files,
        &mut todo_items,
        &mut dir_file_counts,
        &mut file_count,
    )?;

    // Build language info from extensions
    let languages = compute_languages(&ext_counts);

    // Compute directory clusters
    let mut directory_clusters: Vec<DirCluster> = dir_file_counts
        .into_iter()
        .filter(|(_, count)| *count >= 2)
        .map(|(path, count)| DirCluster {
            path,
            file_count: count,
            dominant_language: None,
        })
        .collect();
    directory_clusters.sort_by(|a, b| b.file_count.cmp(&a.file_count));
    directory_clusters.truncate(20);

    // Guess build/test commands
    let likely_build_cmd = guess_build_cmd(&build_files, &languages);
    let likely_test_cmd = guess_test_cmd(&build_files, &languages);

    Ok(RepoScan {
        root: root.to_path_buf(),
        file_count,
        languages,
        build_files,
        test_patterns,
        config_files,
        todo_fixme_hack: todo_items,
        directory_clusters,
        likely_build_cmd,
        likely_test_cmd,
    })
}

#[allow(clippy::too_many_arguments)]
fn walk_dir(
    root: &Path,
    dir: &Path,
    depth: usize,
    max_depth: usize,
    ext_counts: &mut HashMap<String, usize>,
    build_files: &mut Vec<String>,
    test_patterns: &mut Vec<String>,
    config_files: &mut Vec<String>,
    todo_items: &mut Vec<TodoItem>,
    dir_file_counts: &mut HashMap<String, usize>,
    file_count: &mut usize,
) -> Result<()> {
    if depth > max_depth {
        return Ok(());
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return Ok(()),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if path.is_dir() {
            if IGNORE_DIRS.contains(&name.as_str()) || name.starts_with('.') {
                continue;
            }
            walk_dir(
                root,
                &path,
                depth + 1,
                max_depth,
                ext_counts,
                build_files,
                test_patterns,
                config_files,
                todo_items,
                dir_file_counts,
                file_count,
            )?;
        } else if path.is_file() {
            *file_count += 1;

            // Track directory
            if let Some(parent) = path.parent() {
                let rel = parent
                    .strip_prefix(root)
                    .unwrap_or(parent)
                    .to_string_lossy()
                    .to_string();
                *dir_file_counts.entry(rel).or_insert(0) += 1;
            }

            // Track extensions
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                *ext_counts.entry(ext.to_lowercase()).or_insert(0) += 1;
            }

            let rel_path = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();

            // Detect build/config files
            classify_special_file(&name, &rel_path, build_files, test_patterns, config_files);

            // Scan for TODOs in source files (only small files)
            if is_source_file(&name) {
                if let Ok(metadata) = path.metadata() {
                    if metadata.len() < 256_000 {
                        scan_todos(&path, &rel_path, todo_items);
                    }
                }
            }
        }
    }

    Ok(())
}

fn classify_special_file(
    name: &str,
    rel_path: &str,
    build_files: &mut Vec<String>,
    test_patterns: &mut Vec<String>,
    config_files: &mut Vec<String>,
) {
    // Build files
    let build_names = [
        "Cargo.toml",
        "Makefile",
        "CMakeLists.txt",
        "build.gradle",
        "pom.xml",
        "package.json",
        "go.mod",
        "Gemfile",
        "requirements.txt",
        "pyproject.toml",
        "setup.py",
        "build.zig",
        "meson.build",
        "BUILD",
        "Dockerfile",
        "docker-compose.yml",
    ];
    if build_names.contains(&name) {
        build_files.push(rel_path.to_string());
    }

    // Config files
    let config_names = [
        ".env",
        ".env.example",
        "config.toml",
        "config.yaml",
        "config.yml",
        "config.json",
        ".eslintrc",
        ".prettierrc",
        "tsconfig.json",
        "rustfmt.toml",
        "clippy.toml",
        ".gitignore",
    ];
    if config_names.contains(&name)
        || name.ends_with(".config.js")
        || name.ends_with(".config.ts")
    {
        config_files.push(rel_path.to_string());
    }

    // Test patterns
    let lower = name.to_lowercase();
    if lower.contains("test") || lower.contains("spec") || lower.starts_with("test_") {
        test_patterns.push(rel_path.to_string());
    }
}

fn is_source_file(name: &str) -> bool {
    let ext = name.rsplit('.').next().unwrap_or("");
    matches!(
        ext,
        "rs" | "py"
            | "js"
            | "ts"
            | "tsx"
            | "jsx"
            | "go"
            | "c"
            | "cpp"
            | "h"
            | "hpp"
            | "java"
            | "rb"
            | "swift"
            | "kt"
            | "scala"
            | "zig"
            | "lua"
            | "sh"
            | "bash"
            | "zsh"
            | "toml"
            | "yaml"
            | "yml"
    )
}

fn scan_todos(path: &Path, rel_path: &str, items: &mut Vec<TodoItem>) {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return,
    };

    for (i, line) in content.lines().enumerate() {
        let upper = line.to_uppercase();
        let kind = if upper.contains("FIXME") {
            Some(TodoKind::Fixme)
        } else if upper.contains("HACK") {
            Some(TodoKind::Hack)
        } else if upper.contains("XXX") {
            Some(TodoKind::Xxx)
        } else if upper.contains("TODO") {
            Some(TodoKind::Todo)
        } else {
            None
        };

        if let Some(kind) = kind {
            items.push(TodoItem {
                path: rel_path.to_string(),
                line_number: i + 1,
                kind,
                text: line.trim().to_string(),
            });
        }
    }
}

fn compute_languages(ext_counts: &HashMap<String, usize>) -> Vec<LanguageInfo> {
    let lang_map: &[(&str, &[&str])] = &[
        ("Rust", &["rs"]),
        ("Python", &["py"]),
        ("JavaScript", &["js", "jsx", "mjs"]),
        ("TypeScript", &["ts", "tsx"]),
        ("Go", &["go"]),
        ("C", &["c", "h"]),
        ("C++", &["cpp", "hpp", "cc", "cxx"]),
        ("Java", &["java"]),
        ("Ruby", &["rb"]),
        ("Swift", &["swift"]),
        ("Kotlin", &["kt", "kts"]),
        ("Zig", &["zig"]),
        ("Shell", &["sh", "bash", "zsh"]),
        ("Lua", &["lua"]),
        ("TOML", &["toml"]),
        ("YAML", &["yaml", "yml"]),
        ("JSON", &["json"]),
        ("Markdown", &["md"]),
        ("HTML", &["html", "htm"]),
        ("CSS", &["css", "scss", "sass"]),
    ];

    let mut languages = Vec::new();
    for (name, exts) in lang_map {
        let count: usize = exts
            .iter()
            .map(|e| ext_counts.get(*e).copied().unwrap_or(0))
            .sum();
        if count > 0 {
            languages.push(LanguageInfo {
                name: name.to_string(),
                extensions: exts.iter().map(|e| e.to_string()).collect(),
                file_count: count,
            });
        }
    }
    languages.sort_by(|a, b| b.file_count.cmp(&a.file_count));
    languages
}

fn guess_build_cmd(build_files: &[String], languages: &[LanguageInfo]) -> Option<String> {
    for f in build_files {
        let name = f.rsplit('/').next().unwrap_or(f);
        match name {
            "Cargo.toml" => return Some("cargo build".to_string()),
            "package.json" => return Some("npm run build".to_string()),
            "go.mod" => return Some("go build ./...".to_string()),
            "Makefile" => return Some("make".to_string()),
            "CMakeLists.txt" => return Some("cmake --build build".to_string()),
            "build.gradle" => return Some("./gradlew build".to_string()),
            "pom.xml" => return Some("mvn compile".to_string()),
            "pyproject.toml" => return Some("pip install -e .".to_string()),
            "build.zig" => return Some("zig build".to_string()),
            _ => {}
        }
    }
    // Fallback by dominant language
    languages.first().and_then(|l| match l.name.as_str() {
        "Python" => Some("python -m py_compile *.py".to_string()),
        "Ruby" => Some("ruby -c *.rb".to_string()),
        _ => None,
    })
}

fn guess_test_cmd(build_files: &[String], languages: &[LanguageInfo]) -> Option<String> {
    for f in build_files {
        let name = f.rsplit('/').next().unwrap_or(f);
        match name {
            "Cargo.toml" => return Some("cargo test".to_string()),
            "package.json" => return Some("npm test".to_string()),
            "go.mod" => return Some("go test ./...".to_string()),
            "build.gradle" => return Some("./gradlew test".to_string()),
            "pom.xml" => return Some("mvn test".to_string()),
            "pyproject.toml" | "setup.py" => return Some("pytest".to_string()),
            _ => {}
        }
    }
    languages.first().and_then(|l| match l.name.as_str() {
        "Python" => Some("pytest".to_string()),
        "Ruby" => Some("bundle exec rspec".to_string()),
        _ => None,
    })
}
