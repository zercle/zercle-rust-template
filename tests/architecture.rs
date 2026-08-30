//! Executable dependency gates for the clean-architecture layering.
//!
//! Each rule scans the `use crate::…` statements of every source file under
//! `src/` and fails with the violated rule's rationale. Mirrors the Go
//! template's `internal/architecture_test.go`: driving adapters depend on the
//! application port, the use case depends on outbound ports + domain +
//! contract, adapters satisfy ports structurally, and the published contract
//! facade `crate::api` is importable only from outside internal code.
//!
//! Scope note: only `crate::`-rooted imports are checked. Relative imports
//! (`super::…`, sibling modules) are inherently intra-layer and cannot cross
//! feature/layer boundaries, so they are out of scope — same spirit as Go's
//! import-path scan.

use std::fs;
use std::path::{Path, PathBuf};

/// A layering rule: `denied` reports whether the crate-rooted `import` path
/// violates the rule for the module at `module` (path relative to `src/`,
/// extension-less, `mod.rs` collapsed to its directory).
struct Rule {
    name: &'static str,
    why: &'static str,
    denied: fn(module: &str, import: &str) -> bool,
}

fn is_under(path: &str, prefix: &str) -> bool {
    path == prefix || path.starts_with(&format!("{prefix}/"))
}

/// Feature name for a `features/<name>/…` module path.
fn feature_of(module: &str) -> Option<&str> {
    let segs: Vec<&str> = module.split('/').collect();
    if segs.len() >= 2 && segs[0] == "features" {
        Some(segs[1])
    } else {
        None
    }
}

fn rules() -> Vec<Rule> {
    vec![
        Rule {
            name: "published-contract-is-outward-only",
            why: "internal code must not import the api facade; depend on the feature contract modules directly",
            denied: |module, import| !is_under(module, "api") && is_under(import, "api"),
        },
        Rule {
            name: "domain-is-innermost",
            why: "domain may depend on nothing crate-internal (stdlib-adjacent crates only)",
            denied: |module, _import| {
                feature_of(module).is_some()
                    && is_under(
                        module,
                        &format!("features/{}/domain", feature_of(module).unwrap_or_default()),
                    )
            },
        },
        Rule {
            name: "contract-is-leaf",
            why: "the wire contract must stay dependency-free so the published api facade drags in nothing",
            denied: |module, _import| {
                feature_of(module).is_some()
                    && is_under(
                        module,
                        &format!(
                            "features/{}/contract",
                            feature_of(module).unwrap_or_default()
                        ),
                    )
            },
        },
        Rule {
            name: "port-depends-only-on-domain",
            why: "outbound ports may reference only their own feature's domain",
            denied: |module, import| {
                let Some(f) = feature_of(module) else {
                    return false;
                };
                is_under(module, &format!("features/{f}/port"))
                    && !is_under(import, &format!("features/{f}/domain"))
            },
        },
        Rule {
            name: "application-depends-on-domain-port-contract",
            why: "use cases orchestrate their own feature's domain, ports, and wire contract, nothing else",
            denied: |module, import| {
                let Some(f) = feature_of(module) else {
                    return false;
                };
                if !is_under(module, &format!("features/{f}/application")) {
                    return false;
                }
                !(is_under(import, &format!("features/{f}/domain"))
                    || is_under(import, &format!("features/{f}/port"))
                    || is_under(import, &format!("features/{f}/contract"))
                    || is_under(import, &format!("features/{f}/application")))
            },
        },
        Rule {
            name: "driven-adapters-ignore-application",
            why: "adapter/driven satisfies ports structurally and must not know about the application layer or driving adapters",
            denied: |module, import| {
                let Some(f) = feature_of(module) else {
                    return false;
                };
                if !is_under(module, &format!("features/{f}/adapter/driven")) {
                    return false;
                }
                is_under(import, &format!("features/{f}/application"))
                    || import.contains("/adapter/driving")
            },
        },
        Rule {
            name: "driving-adapters-ignore-ports-and-driven-adapters",
            why: "adapter/driving talks to the application port only, never to outbound ports or other adapters",
            denied: |module, import| {
                let Some(f) = feature_of(module) else {
                    return false;
                };
                if !is_under(module, &format!("features/{f}/adapter/driving")) {
                    return false;
                }
                is_under(import, &format!("features/{f}/port"))
                    || import.contains("/adapter/driven")
            },
        },
        Rule {
            name: "platform-ignores-features",
            why: "cross-cutting platform code must stay feature-agnostic; features depend on platform, never the reverse",
            denied: |module, import| is_under(module, "platform") && is_under(import, "features"),
        },
    ]
}

/// Collapse a file path under `src/` to its module path: extension stripped,
/// `mod.rs` replaced by its directory.
fn module_path(src_root: &Path, file: &Path) -> String {
    let rel = file
        .strip_prefix(src_root)
        .expect("file is under src root")
        .to_string_lossy()
        .into_owned();
    let rel = rel.strip_suffix(".rs").unwrap_or(&rel);
    let rel = rel.strip_suffix("/mod").unwrap_or(rel);
    rel.to_string()
}

/// Remove `//` line comments so commented-out code cannot trip the scanner.
/// (Block comments are not used for code in this codebase's style.)
fn strip_line_comments(text: &str) -> String {
    text.lines()
        .map(|line| match line.find("//") {
            Some(idx) if !line[..idx].contains('"') => &line[..idx],
            _ => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Extract all `crate::`-rooted import paths from a source text, expanding
/// braced use-trees (`use crate::a::{b::{c}, d};`) and stripping `as` renames
/// and glob markers.
///
/// All slice indices derive from `str::find` results, so multibyte source
/// text (identifiers in test fixtures, CJK comments) is handled safely.
fn crate_imports(text: &str) -> Vec<String> {
    let text = strip_line_comments(text);
    let mut imports = Vec::new();
    let mut search_from = 0usize;
    while let Some(rel) = text[search_from..].find("use crate::") {
        let stmt_start = search_from + rel + "use ".len();
        let stmt_end = text[stmt_start..]
            .find(';')
            .map_or(text.len(), |e| stmt_start + e);
        let tree = text[stmt_start..stmt_end].trim();
        let tree = tree.strip_prefix("crate::").unwrap_or(tree);
        expand_use_tree("", tree, &mut imports);
        search_from = stmt_end.max(stmt_start + 1);
    }
    imports
}

/// Recursively expand a use-tree (`a::b`, `{x, y}`, `a::{b::{c}, d}`,
/// `x as y`, `prelude::*`) into concrete paths, prefixing each with `prefix`.
fn expand_use_tree(prefix: &str, tree: &str, out: &mut Vec<String>) {
    // Split on top-level commas.
    let mut depth = 0i32;
    let mut start = 0usize;
    let mut parts: Vec<&str> = Vec::new();
    for (i, c) in tree.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(&tree[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&tree[start..]);

    for part in parts {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        // Locate the first top-level `::{` group.
        let bytes = part.as_bytes();
        let mut group: Option<(usize, usize)> = None; // (head_end, close_index)
        let mut d = 0i32;
        let mut j = 0usize;
        while j < bytes.len() {
            match bytes[j] {
                b'{' => d += 1,
                b'}' => d -= 1,
                b':' if d == 0
                    && j + 2 < bytes.len()
                    && bytes[j + 1] == b':'
                    && bytes[j + 2] == b'{' =>
                {
                    let mut dd = 1i32;
                    let mut k = j + 3;
                    while k < bytes.len() && dd > 0 {
                        match bytes[k] {
                            b'{' => dd += 1,
                            b'}' => dd -= 1,
                            _ => {}
                        }
                        k += 1;
                    }
                    group = Some((j, k - 1));
                    break;
                }
                _ => {}
            }
            j += 1;
        }

        if let Some((head_end, close)) = group {
            let head = &part[..head_end];
            let inner = &part[head_end + 3..close];
            expand_use_tree(&format!("{prefix}{head}::"), inner, out);
        } else if part.starts_with('{') && part.ends_with('}') {
            expand_use_tree(prefix, &part[1..part.len() - 1], out);
        } else {
            let leaf = part.split(" as ").next().unwrap_or(part).trim();
            let leaf = leaf.strip_suffix('*').unwrap_or(leaf);
            if !leaf.is_empty() {
                // Emit module paths in slash form (the form the rule table
                // compares against): `a::b::C` → `a/b/C`.
                out.push(format!("{prefix}{leaf}").replace("::", "/"));
            }
        }
    }
}

/// Recursively collect `*.rs` files under `dir`.
fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("read dir {}: {e}", dir.display()));
    for entry in entries {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn architecture_rules_hold() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let src_root = Path::new(&manifest).join("src");

    let mut files = Vec::new();
    collect_rs_files(&src_root, &mut files);
    assert!(
        files.len() >= 20,
        "expected the full src tree, found {}",
        files.len()
    );
    files.sort();

    let rule_table = rules();
    let mut violations: Vec<String> = Vec::new();

    for file in &files {
        let module = module_path(&src_root, file);
        let text = &fs::read_to_string(file).expect("read source");
        for import in crate_imports(text) {
            for rule in &rule_table {
                if (rule.denied)(&module, &import) {
                    violations.push(format!(
                        "{}: module `{}` violates {}: imports `crate::{}` ({})",
                        file.display(),
                        module,
                        rule.name,
                        import,
                        rule.why
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "clean-architecture dependency rules violated:\n  {}",
        violations.join("\n  ")
    );
}

#[cfg(test)]
mod scanner_tests {
    use super::*;

    #[test]
    fn expands_plain_paths() {
        assert_eq!(
            crate_imports("use crate::platform::config::Config;"),
            vec!["platform/config/Config".to_string()]
        );
    }

    #[test]
    fn expands_braced_groups_with_rename() {
        let got = crate_imports(
            "use crate::platform::{middleware::{access_log, cors}, telemetry::metrics_body as render_metrics};",
        );
        assert_eq!(
            got,
            vec![
                "platform/middleware/access_log".to_string(),
                "platform/middleware/cors".to_string(),
                "platform/telemetry/metrics_body".to_string(),
            ]
        );
    }

    #[test]
    fn ignores_comments_and_relative_imports() {
        let got = crate_imports("// use crate::platform::db;\nuse super::http;\nuse sqlx::PgPool;");
        assert!(got.is_empty(), "got {got:?}");
    }

    #[test]
    fn module_paths_collapse_mod_rs() {
        let root = Path::new("/x/src");
        assert_eq!(
            module_path(root, Path::new("/x/src/platform/server/mod.rs")),
            "platform/server"
        );
        assert_eq!(module_path(root, Path::new("/x/src/app.rs")), "app");
    }
}
