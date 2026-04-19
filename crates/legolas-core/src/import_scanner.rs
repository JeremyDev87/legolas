use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use once_cell::sync::Lazy;
use regex::Regex;

use crate::{
    aliases::{AliasConfig, AliasTarget},
    confidence::score_tree_shaking_warning,
    error::Result,
    models::TreeShakingWarning,
    FindingAnalysisSource, FindingEvidence, FindingMetadata, LegolasError,
};

const IGNORED_DIRECTORIES: &[&str] = &[
    ".git",
    "node_modules",
    "dist",
    "build",
    ".next",
    ".turbo",
    "coverage",
    ".output",
    "test",
    "tests",
    "__tests__",
];

const SOURCE_FILE_SUFFIXES: &[&str] = &[
    ".js", ".jsx", ".ts", ".tsx", ".cjs", ".cjsx", ".cts", ".ctsx", ".mjs", ".mjsx", ".mts",
    ".mtsx", ".vue", ".svelte",
];

static SCRIPT_BLOCK_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?is)<script\b[^>]*>(.*?)</script>").expect("valid script regex"));

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImportedPackageRecord {
    pub name: String,
    pub files: Vec<String>,
    pub static_files: Vec<String>,
    pub dynamic_files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceAnalysis {
    pub by_package: BTreeMap<String, ImportedPackageRecord>,
    pub imported_packages: Vec<ImportedPackageRecord>,
    pub dynamic_import_count: usize,
    pub tree_shaking_warnings: Vec<TreeShakingWarning>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportKind {
    Static,
    Dynamic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ImportEntry {
    kind: ImportKind,
    specifier: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct ScannedSourceFile {
    imports: Vec<ImportEntry>,
    tree_shaking_hints: Vec<TreeShakingWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedToken {
    import_entry: Option<ImportEntry>,
    tree_shaking_hint: Option<TreeShakingWarning>,
    next_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedArgument {
    specifier: String,
    next_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedStringLiteral {
    value: String,
    next_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct PackageAccumulator {
    name: String,
    files: BTreeSet<String>,
    static_files: BTreeSet<String>,
    dynamic_files: BTreeSet<String>,
}

impl PackageAccumulator {
    fn new(name: String) -> Self {
        Self {
            name,
            ..Self::default()
        }
    }

    fn into_record(self) -> ImportedPackageRecord {
        ImportedPackageRecord {
            name: self.name,
            files: self.files.into_iter().collect(),
            static_files: self.static_files.into_iter().collect(),
            dynamic_files: self.dynamic_files.into_iter().collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WarningAccumulator {
    key: String,
    package_name: String,
    message: String,
    recommendation: String,
    estimated_kb: usize,
    files: BTreeSet<String>,
    finding: FindingMetadata,
}

pub fn collect_source_files<P: AsRef<Path>>(project_root: P) -> Result<Vec<PathBuf>> {
    let project_root = project_root.as_ref();
    ensure_directory_exists(project_root)?;

    let mut files = Vec::new();
    walk(project_root, &mut files)?;
    files.sort();
    Ok(files)
}

pub fn scan_imports<P: AsRef<Path>>(
    project_root: P,
    source_files: &[PathBuf],
) -> Result<SourceAnalysis> {
    scan_imports_with_aliases(project_root, source_files, None)
}

pub fn scan_imports_with_aliases<P: AsRef<Path>>(
    project_root: P,
    source_files: &[PathBuf],
    alias_config: Option<&AliasConfig>,
) -> Result<SourceAnalysis> {
    let project_root = project_root.as_ref();
    ensure_directory_exists(project_root)?;

    let mut ordered_files = source_files.to_vec();
    ordered_files.sort();

    let mut by_package: BTreeMap<String, PackageAccumulator> = BTreeMap::new();
    let mut tree_shaking_observations = Vec::new();
    let mut dynamic_import_count = 0;

    for absolute_path in ordered_files {
        let contents = fs::read_to_string(&absolute_path)?;
        let relative_path = to_posix_relative(project_root, &absolute_path);
        let scannable_contents = get_scannable_contents(&absolute_path, &contents);
        let scanned =
            scan_source_file(&scannable_contents, supports_jsx_text_guard(&absolute_path));

        for entry in scanned.imports {
            if entry.kind == ImportKind::Dynamic {
                dynamic_import_count += 1;
            }

            if is_local_alias_import(&entry.specifier, alias_config) {
                continue;
            }

            let Some(package_name) = normalize_package_name(&entry.specifier) else {
                continue;
            };

            let record = by_package
                .entry(package_name.clone())
                .or_insert_with(|| PackageAccumulator::new(package_name));

            record.files.insert(relative_path.clone());
            if entry.kind == ImportKind::Dynamic {
                record.dynamic_files.insert(relative_path.clone());
            } else {
                record.static_files.insert(relative_path.clone());
            }
        }

        for mut hint in scanned.tree_shaking_hints {
            hint.files = vec![relative_path.clone()];
            hint.finding =
                build_tree_shaking_finding(&hint.key, &hint.package_name, &relative_path);
            tree_shaking_observations.push(hint);
        }
    }

    let by_package = by_package
        .into_iter()
        .map(|(name, record)| (name, record.into_record()))
        .collect::<BTreeMap<_, _>>();
    let imported_packages = by_package.values().cloned().collect();

    Ok(SourceAnalysis {
        by_package,
        imported_packages,
        dynamic_import_count,
        tree_shaking_warnings: merge_tree_shaking_warnings(tree_shaking_observations),
    })
}

fn ensure_directory_exists(path: &Path) -> Result<()> {
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Err(LegolasError::PathNotFound(path.display().to_string()))
        }
        Err(error) => Err(error.into()),
    }
}

fn walk(current_path: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let mut entries = fs::read_dir(current_path)?.collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let absolute_path = entry.path();

        if entry.file_type()?.is_dir() {
            let name = entry.file_name();
            if IGNORED_DIRECTORIES.iter().any(|ignored| name == *ignored) {
                continue;
            }
            walk(&absolute_path, files)?;
            continue;
        }

        if is_source_file(&entry.file_name().to_string_lossy()) {
            files.push(absolute_path);
        }
    }

    Ok(())
}

fn is_source_file(file_name: &str) -> bool {
    SOURCE_FILE_SUFFIXES
        .iter()
        .any(|suffix| file_name.ends_with(suffix))
}

fn scan_source_file(contents: &str, jsx_text_guard: bool) -> ScannedSourceFile {
    let mut imports = Vec::new();
    let mut tree_shaking_hints = Vec::new();
    let mut index = 0;

    while index < contents.len() {
        let Some(character) = current_char(contents, index) else {
            break;
        };

        if character == '/' && peek_char(contents, index + 1) == Some('/') {
            index = skip_line_comment(contents, index);
            continue;
        }

        if character == '/' && peek_char(contents, index + 1) == Some('*') {
            index = skip_block_comment(contents, index);
            continue;
        }

        if character == '\'' || character == '"' {
            index = skip_quoted_string(contents, index, character);
            continue;
        }

        if character == '`' {
            index = skip_template_string(contents, index);
            continue;
        }

        if !is_identifier_start(character) {
            index = advance_one(contents, index);
            continue;
        }

        let token = read_identifier(contents, index);

        if jsx_text_guard && is_inside_likely_jsx_text(contents, index) {
            index += token.len();
            continue;
        }

        if token == "import" {
            if let Some(parsed) = try_parse_import(contents, index) {
                if let Some(import_entry) = parsed.import_entry {
                    imports.push(import_entry);
                }
                if let Some(tree_shaking_hint) = parsed.tree_shaking_hint {
                    tree_shaking_hints.push(tree_shaking_hint);
                }
                index = parsed.next_index;
                continue;
            }
        }

        if token == "export" {
            if let Some(parsed) = try_parse_export_from(contents, index) {
                if let Some(import_entry) = parsed.import_entry {
                    imports.push(import_entry);
                }
                index = parsed.next_index;
                continue;
            }
        }

        if token == "require" {
            if let Some(parsed) = try_parse_require(contents, index) {
                if let Some(import_entry) = parsed.import_entry {
                    imports.push(import_entry);
                }
                index = parsed.next_index;
                continue;
            }
        }

        index += token.len();
    }

    ScannedSourceFile {
        imports,
        tree_shaking_hints,
    }
}

fn normalize_package_name(specifier: &str) -> Option<String> {
    if specifier.is_empty()
        || specifier.starts_with("node:")
        || specifier.starts_with('.')
        || specifier.starts_with('/')
        || specifier.starts_with("~/")
        || specifier.starts_with("@/")
        || specifier.starts_with('#')
        || specifier.starts_with("virtual:")
    {
        return None;
    }

    if specifier.starts_with('@') {
        let mut segments = specifier.split('/');
        let scope = segments.next()?;
        let name = segments.next()?;
        if scope.is_empty() || name.is_empty() {
            return None;
        }
        return Some(format!("{scope}/{name}"));
    }

    Some(
        specifier
            .split('/')
            .next()
            .expect("split always returns at least one segment")
            .to_string(),
    )
}

fn is_local_alias_import(specifier: &str, alias_config: Option<&AliasConfig>) -> bool {
    let Some(alias_config) = alias_config else {
        return false;
    };

    alias_config.rules.iter().any(|rule| {
        match_alias_rule(rule, specifier).is_some_and(|capture| {
            rule.replacement_targets
                .iter()
                .any(|target| alias_target_exists(target, capture))
        })
    })
}

fn match_alias_rule<'a>(rule: &crate::aliases::AliasRule, specifier: &'a str) -> Option<&'a str> {
    if !rule.wildcard {
        return (specifier == rule.specifier_prefix).then_some("");
    }

    let capture = specifier.strip_prefix(&rule.specifier_prefix)?;
    let suffix = wildcard_suffix(&rule.pattern);

    if suffix.is_empty() {
        Some(capture)
    } else {
        capture.strip_suffix(suffix)
    }
}

fn wildcard_suffix(pattern: &str) -> &str {
    pattern
        .split_once('*')
        .map(|(_, suffix)| suffix)
        .unwrap_or("")
}

fn alias_target_exists(target: &AliasTarget, capture: &str) -> bool {
    resolved_alias_candidates(target, capture)
        .into_iter()
        .any(|candidate| is_local_alias_candidate(&candidate))
}

fn alias_candidate_path(target: &AliasTarget, capture: &str) -> PathBuf {
    if !target.pattern.contains('*') {
        return target.path_candidate.clone();
    }

    let relative_tail = format!("{capture}{}", wildcard_suffix(&target.pattern));
    let relative_tail = relative_tail.trim_start_matches('/');

    if relative_tail.is_empty() {
        target.path_candidate.clone()
    } else {
        target.path_candidate.join(Path::new(relative_tail))
    }
}

fn resolved_alias_candidates(target: &AliasTarget, capture: &str) -> Vec<PathBuf> {
    let candidate = alias_candidate_path(target, capture);
    let mut candidates = vec![candidate.clone()];

    if candidate.extension().is_none() {
        let base = candidate.to_string_lossy();
        for suffix in SOURCE_FILE_SUFFIXES {
            candidates.push(PathBuf::from(format!("{base}{suffix}")));
        }
    }

    for suffix in SOURCE_FILE_SUFFIXES {
        candidates.push(candidate.join(format!("index{suffix}")));
    }

    candidates
}

fn path_exists(path: &Path) -> bool {
    fs::metadata(path).is_ok()
}

fn is_local_alias_candidate(path: &Path) -> bool {
    if !path_exists(path) || is_package_install_path(path) {
        return false;
    }

    match fs::canonicalize(path) {
        Ok(canonical) => !is_package_install_path(&canonical),
        Err(_) => true,
    }
}

fn is_package_install_path(path: &Path) -> bool {
    path.components().any(|component| match component {
        std::path::Component::Normal(segment) => segment == "node_modules",
        _ => false,
    })
}

fn merge_tree_shaking_warnings(warnings: Vec<TreeShakingWarning>) -> Vec<TreeShakingWarning> {
    let mut merged = Vec::<WarningAccumulator>::new();
    let mut index_by_key = BTreeMap::<String, usize>::new();

    for warning in warnings {
        let map_key = format!("{}:{}", warning.key, warning.package_name);
        if let Some(existing_index) = index_by_key.get(&map_key).copied() {
            let existing = &mut merged[existing_index];
            for file in warning.files {
                existing.files.insert(file);
            }
            existing.estimated_kb = existing.estimated_kb.max(warning.estimated_kb);
            merge_finding_metadata(&mut existing.finding, warning.finding);
            continue;
        }

        index_by_key.insert(map_key, merged.len());
        merged.push(WarningAccumulator {
            key: warning.key,
            package_name: warning.package_name,
            message: warning.message,
            recommendation: warning.recommendation,
            estimated_kb: warning.estimated_kb,
            files: warning.files.into_iter().collect(),
            finding: warning.finding,
        });
    }

    merged
        .into_iter()
        .map(|warning| TreeShakingWarning {
            key: warning.key,
            package_name: warning.package_name,
            message: warning.message,
            recommendation: warning.recommendation,
            estimated_kb: warning.estimated_kb,
            files: warning.files.into_iter().collect(),
            finding: warning.finding,
        })
        .collect()
}

fn merge_finding_metadata(existing: &mut FindingMetadata, mut incoming: FindingMetadata) {
    if existing.finding_id.is_none() {
        existing.finding_id = incoming.finding_id.take();
    }

    if existing.analysis_source.is_none() {
        existing.analysis_source = incoming.analysis_source.take();
    }

    match (existing.confidence, incoming.confidence.take()) {
        (Some(current), Some(next)) if next > current => {
            existing.confidence = Some(next);
        }
        (None, Some(next)) => {
            existing.confidence = Some(next);
        }
        _ => {}
    }

    existing.evidence.append(&mut incoming.evidence);
    normalize_finding_evidence(&mut existing.evidence);
}

fn normalize_finding_evidence(evidence: &mut Vec<FindingEvidence>) {
    evidence.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then(left.file.cmp(&right.file))
            .then(left.specifier.cmp(&right.specifier))
            .then(left.detail.cmp(&right.detail))
    });
    evidence.dedup_by(|left, right| {
        left.kind == right.kind
            && left.file == right.file
            && left.specifier == right.specifier
            && left.detail == right.detail
    });
}

fn get_scannable_contents(file_path: &Path, contents: &str) -> String {
    match extension(file_path) {
        ".vue" | ".svelte" => extract_script_blocks(contents),
        _ => contents.to_string(),
    }
}

fn extract_script_blocks(contents: &str) -> String {
    SCRIPT_BLOCK_PATTERN
        .captures_iter(contents)
        .filter_map(|captures| captures.get(1).map(|value| value.as_str()))
        .collect::<Vec<_>>()
        .join("\n")
}

fn supports_jsx_text_guard(file_path: &Path) -> bool {
    matches!(
        extension(file_path),
        ".js" | ".jsx" | ".ts" | ".tsx" | ".mjs" | ".cjs"
    )
}

fn try_parse_import(contents: &str, start_index: usize) -> Option<ParsedToken> {
    if !has_token_boundary(contents, start_index, "import") {
        return None;
    }

    let index = skip_trivia(contents, start_index + "import".len());
    let character = current_char(contents, index)?;

    if character == '(' {
        let parsed_argument = parse_quoted_argument(contents, index)?;
        return Some(ParsedToken {
            import_entry: Some(ImportEntry {
                kind: ImportKind::Dynamic,
                specifier: parsed_argument.specifier,
            }),
            tree_shaking_hint: None,
            next_index: parsed_argument.next_index,
        });
    }

    if character == '\'' || character == '"' {
        let parsed_string = read_string_literal(contents, index)?;
        return Some(ParsedToken {
            import_entry: Some(ImportEntry {
                kind: ImportKind::Static,
                specifier: parsed_string.value,
            }),
            tree_shaking_hint: None,
            next_index: parsed_string.next_index,
        });
    }

    if character == '.' {
        return None;
    }

    let from_index = find_keyword(contents, index, "from")?;
    let clause = contents[index..from_index].trim();
    let parsed_specifier =
        read_string_literal(contents, skip_trivia(contents, from_index + "from".len()))?;

    if is_type_only_clause(clause) {
        return Some(ParsedToken {
            import_entry: None,
            tree_shaking_hint: None,
            next_index: parsed_specifier.next_index,
        });
    }

    let specifier = parsed_specifier.value;
    let tree_shaking_hint = build_tree_shaking_hint(&specifier, clause);

    Some(ParsedToken {
        tree_shaking_hint,
        import_entry: Some(ImportEntry {
            kind: ImportKind::Static,
            specifier,
        }),
        next_index: parsed_specifier.next_index,
    })
}

fn try_parse_export_from(contents: &str, start_index: usize) -> Option<ParsedToken> {
    if !has_token_boundary(contents, start_index, "export") {
        return None;
    }

    let search_start = skip_trivia(contents, start_index + "export".len());
    let from_index = find_keyword(contents, search_start, "from")?;
    let parsed_specifier =
        read_string_literal(contents, skip_trivia(contents, from_index + "from".len()))?;
    let clause = contents[search_start..from_index].trim();

    if is_type_only_clause(clause) {
        return Some(ParsedToken {
            import_entry: None,
            tree_shaking_hint: None,
            next_index: parsed_specifier.next_index,
        });
    }

    Some(ParsedToken {
        import_entry: Some(ImportEntry {
            kind: ImportKind::Static,
            specifier: parsed_specifier.value,
        }),
        tree_shaking_hint: None,
        next_index: parsed_specifier.next_index,
    })
}

fn try_parse_require(contents: &str, start_index: usize) -> Option<ParsedToken> {
    if !has_token_boundary(contents, start_index, "require") {
        return None;
    }

    let parsed_argument = parse_quoted_argument(
        contents,
        skip_trivia(contents, start_index + "require".len()),
    )?;
    Some(ParsedToken {
        import_entry: Some(ImportEntry {
            kind: ImportKind::Static,
            specifier: parsed_argument.specifier,
        }),
        tree_shaking_hint: None,
        next_index: parsed_argument.next_index,
    })
}

fn build_tree_shaking_hint(specifier: &str, clause: &str) -> Option<TreeShakingWarning> {
    let normalized_clause = normalize_whitespace(clause);

    if is_namespace_import_clause(&normalized_clause) && is_namespace_sensitive_package(specifier) {
        return Some(TreeShakingWarning {
            key: "namespace-ui-import".to_string(),
            package_name: specifier.to_string(),
            message: "Namespace imports pull large symbol sets into a single module graph."
                .to_string(),
            recommendation: "Import only the symbols you need from direct subpaths.".to_string(),
            estimated_kb: 35,
            files: Vec::new(),
            finding: Default::default(),
        });
    }

    if specifier == "lodash" && !normalized_clause.is_empty() {
        return Some(TreeShakingWarning {
            key: "lodash-root-import".to_string(),
            package_name: "lodash".to_string(),
            message: "Root lodash imports often keep more code than expected in client bundles."
                .to_string(),
            recommendation: "Prefer per-method imports or lodash-es.".to_string(),
            estimated_kb: 26,
            files: Vec::new(),
            finding: Default::default(),
        });
    }

    if specifier == "react-icons" {
        return Some(TreeShakingWarning {
            key: "react-icons-root-import".to_string(),
            package_name: "react-icons".to_string(),
            message: "Root react-icons imports can make tree shaking unreliable.".to_string(),
            recommendation: "Import from the specific icon pack path instead.".to_string(),
            estimated_kb: 22,
            files: Vec::new(),
            finding: Default::default(),
        });
    }

    None
}

fn build_tree_shaking_finding(
    warning_key: &str,
    package_name: &str,
    relative_path: &str,
) -> FindingMetadata {
    let mut evidence = FindingEvidence::new("source-file")
        .with_file(relative_path.to_string())
        .with_specifier(package_name.to_string());

    if let Some(detail) = tree_shaking_evidence_detail(warning_key) {
        evidence = evidence.with_detail(detail);
    }

    FindingMetadata::new(
        format!("tree-shaking:{warning_key}"),
        FindingAnalysisSource::SourceImport,
    )
    .with_confidence(score_tree_shaking_warning())
    .with_evidence([evidence])
}

fn tree_shaking_evidence_detail(warning_key: &str) -> Option<&'static str> {
    match warning_key {
        "namespace-ui-import" => Some("namespace import"),
        "lodash-root-import" | "react-icons-root-import" => Some("root package import"),
        _ => None,
    }
}

fn is_type_only_clause(clause: &str) -> bool {
    let normalized_clause = normalize_whitespace(clause);

    if normalized_clause.is_empty() {
        return false;
    }

    if normalized_clause.starts_with("type ") {
        return true;
    }

    if !normalized_clause.starts_with('{') || !normalized_clause.ends_with('}') {
        return false;
    }

    let specifiers = normalized_clause[1..normalized_clause.len() - 1]
        .split(',')
        .map(str::trim)
        .filter(|specifier| !specifier.is_empty())
        .collect::<Vec<_>>();

    !specifiers.is_empty()
        && specifiers
            .iter()
            .all(|specifier| specifier.starts_with("type "))
}

fn is_namespace_sensitive_package(specifier: &str) -> bool {
    matches!(specifier, "lodash" | "lucide-react" | "@mui/icons-material")
}

fn is_namespace_import_clause(clause: &str) -> bool {
    if let Some(rest) = clause.strip_prefix("* as ") {
        !rest.is_empty()
            && rest
                .chars()
                .all(|character| character == '_' || character.is_ascii_alphanumeric())
    } else {
        false
    }
}

fn parse_quoted_argument(contents: &str, start_index: usize) -> Option<ParsedArgument> {
    let mut index = skip_trivia(contents, start_index);
    if current_char(contents, index)? != '(' {
        return None;
    }

    index = skip_trivia(contents, index + 1);
    let parsed_string = read_string_literal(contents, index)?;
    Some(ParsedArgument {
        specifier: parsed_string.value,
        next_index: parsed_string.next_index,
    })
}

fn read_string_literal(contents: &str, start_index: usize) -> Option<ParsedStringLiteral> {
    let quote = current_char(contents, start_index)?;
    if quote != '\'' && quote != '"' {
        return None;
    }

    let mut value = String::new();
    let mut index = start_index + quote.len_utf8();

    while index < contents.len() {
        let character = current_char(contents, index)?;

        if character == '\\' {
            let next_index = advance_one(contents, index);
            if next_index >= contents.len() {
                return None;
            }

            let next_character = current_char(contents, next_index)?;
            value.push(next_character);
            index = advance_one(contents, next_index);
            continue;
        }

        if character == quote {
            return Some(ParsedStringLiteral {
                value,
                next_index: index + quote.len_utf8(),
            });
        }

        value.push(character);
        index = advance_one(contents, index);
    }

    None
}

fn find_keyword(contents: &str, start_index: usize, keyword: &str) -> Option<usize> {
    let mut index = start_index;
    let mut depth = 0usize;

    while index < contents.len() {
        let character = current_char(contents, index)?;

        if character == '/' && peek_char(contents, index + 1) == Some('/') {
            index = skip_line_comment(contents, index);
            continue;
        }

        if character == '/' && peek_char(contents, index + 1) == Some('*') {
            index = skip_block_comment(contents, index);
            continue;
        }

        if character == '\'' || character == '"' {
            index = skip_quoted_string(contents, index, character);
            continue;
        }

        if character == '`' {
            index = skip_template_string(contents, index);
            continue;
        }

        if matches!(character, '{' | '(' | '[') {
            depth += 1;
            index = advance_one(contents, index);
            continue;
        }

        if matches!(character, '}' | ')' | ']') {
            depth = depth.saturating_sub(1);
            index = advance_one(contents, index);
            continue;
        }

        if depth == 0 && is_keyword_at(contents, index, keyword) {
            return Some(index);
        }

        if depth == 0 && character == ';' {
            return None;
        }

        index = advance_one(contents, index);
    }

    None
}

fn skip_trivia(contents: &str, start_index: usize) -> usize {
    let mut index = start_index;

    while index < contents.len() {
        let Some(character) = current_char(contents, index) else {
            break;
        };

        if character.is_whitespace() {
            index = advance_one(contents, index);
            continue;
        }

        if character == '/' && peek_char(contents, index + 1) == Some('/') {
            index = skip_line_comment(contents, index);
            continue;
        }

        if character == '/' && peek_char(contents, index + 1) == Some('*') {
            index = skip_block_comment(contents, index);
            continue;
        }

        return index;
    }

    index
}

fn skip_line_comment(contents: &str, start_index: usize) -> usize {
    let mut index = start_index + 2;

    while index < contents.len() {
        let Some(character) = current_char(contents, index) else {
            break;
        };

        if character == '\n' {
            break;
        }

        index = advance_one(contents, index);
    }

    index
}

fn skip_block_comment(contents: &str, start_index: usize) -> usize {
    let mut index = start_index + 2;

    while index + 1 < contents.len() {
        if current_char(contents, index) == Some('*') && peek_char(contents, index + 1) == Some('/')
        {
            return index + 2;
        }
        index = advance_one(contents, index);
    }

    contents.len()
}

fn skip_quoted_string(contents: &str, start_index: usize, quote: char) -> usize {
    let mut index = start_index + quote.len_utf8();

    while index < contents.len() {
        let Some(character) = current_char(contents, index) else {
            break;
        };

        if character == '\\' {
            let next_index = advance_one(contents, index);
            if next_index >= contents.len() {
                return contents.len();
            }
            index = advance_one(contents, next_index);
            continue;
        }

        if character == quote {
            return index + quote.len_utf8();
        }

        index = advance_one(contents, index);
    }

    contents.len()
}

fn skip_template_string(contents: &str, start_index: usize) -> usize {
    let mut index = start_index + 1;

    while index < contents.len() {
        let Some(character) = current_char(contents, index) else {
            break;
        };

        if character == '\\' {
            let next_index = advance_one(contents, index);
            if next_index >= contents.len() {
                return contents.len();
            }
            index = advance_one(contents, next_index);
            continue;
        }

        if character == '`' {
            return index + 1;
        }

        if character == '$' && peek_char(contents, index + 1) == Some('{') {
            index = skip_balanced_expression(contents, index + 2);
            continue;
        }

        index = advance_one(contents, index);
    }

    contents.len()
}

fn skip_balanced_expression(contents: &str, start_index: usize) -> usize {
    let mut stack = vec!['}'];
    let mut index = start_index;

    while index < contents.len() && !stack.is_empty() {
        let Some(character) = current_char(contents, index) else {
            break;
        };

        if character == '/' && peek_char(contents, index + 1) == Some('/') {
            index = skip_line_comment(contents, index);
            continue;
        }

        if character == '/' && peek_char(contents, index + 1) == Some('*') {
            index = skip_block_comment(contents, index);
            continue;
        }

        if character == '\'' || character == '"' {
            index = skip_quoted_string(contents, index, character);
            continue;
        }

        if character == '`' {
            index = skip_template_string(contents, index);
            continue;
        }

        if matches!(character, '{' | '(' | '[') {
            stack.push(get_closing_character(character));
            index = advance_one(contents, index);
            continue;
        }

        if Some(character) == stack.last().copied() {
            stack.pop();
            index = advance_one(contents, index);
            continue;
        }

        index = advance_one(contents, index);
    }

    index
}

fn get_closing_character(open_character: char) -> char {
    match open_character {
        '{' => '}',
        '(' => ')',
        _ => ']',
    }
}

fn has_token_boundary(contents: &str, start_index: usize, token: &str) -> bool {
    let previous_character = previous_char(contents, start_index);
    let next_character = current_char(contents, start_index + token.len());

    let valid_previous = previous_character
        .map(|character| !is_identifier_character(character) && character != '.')
        .unwrap_or(true);
    let valid_next = next_character
        .map(|character| !is_identifier_character(character))
        .unwrap_or(true);

    valid_previous && valid_next
}

fn is_keyword_at(contents: &str, start_index: usize, keyword: &str) -> bool {
    contents[start_index..].starts_with(keyword)
        && has_token_boundary(contents, start_index, keyword)
}

fn is_inside_likely_jsx_text(contents: &str, start_index: usize) -> bool {
    let Some(previous_non_whitespace_index) = find_previous_non_whitespace(contents, start_index)
    else {
        return false;
    };

    if current_char(contents, previous_non_whitespace_index) != Some('>') {
        return false;
    }

    let Some(previous_tag_start) = contents[..previous_non_whitespace_index + 1].rfind('<') else {
        return false;
    };
    let previous_tag = &contents[previous_tag_start..previous_non_whitespace_index + 1];
    if !is_likely_jsx_tag(previous_tag) {
        return false;
    }

    let mut next_boundary_index = start_index;
    while next_boundary_index < contents.len() {
        let Some(character) = current_char(contents, next_boundary_index) else {
            break;
        };

        if matches!(character, '<' | '{' | '}' | ';' | '\n' | '\r') {
            break;
        }

        next_boundary_index = advance_one(contents, next_boundary_index);
    }

    if current_char(contents, next_boundary_index) != Some('<') {
        return false;
    }

    let Some(relative_end) = contents[next_boundary_index..].find('>') else {
        return false;
    };
    let next_tag_end = next_boundary_index + relative_end;
    let next_tag = &contents[next_boundary_index..next_tag_end + 1];

    is_likely_jsx_tag(next_tag)
}

fn find_previous_non_whitespace(contents: &str, start_index: usize) -> Option<usize> {
    contents[..start_index]
        .char_indices()
        .rev()
        .find_map(|(index, character)| (!character.is_whitespace()).then_some(index))
}

fn is_likely_jsx_tag(tag_text: &str) -> bool {
    if tag_text == "<>" || tag_text == "</>" {
        return true;
    }

    let mut characters = tag_text.chars();
    if characters.next() != Some('<') {
        return false;
    }

    let next = characters.next();
    let first_tag_character = match next {
        Some('/') => characters.next(),
        other => other,
    };

    matches!(first_tag_character, Some(character) if character.is_ascii_alphabetic())
        && tag_text.ends_with('>')
        && !tag_text[1..tag_text.len() - 1]
            .chars()
            .any(|character| character == '<' || character == '>')
}

fn read_identifier(contents: &str, start_index: usize) -> &str {
    let mut index = start_index;
    while let Some(character) = current_char(contents, index) {
        if !is_identifier_character(character) {
            break;
        }
        index = advance_one(contents, index);
    }
    &contents[start_index..index]
}

fn is_identifier_start(character: char) -> bool {
    character == '$' || character == '_' || character.is_ascii_alphabetic()
}

fn is_identifier_character(character: char) -> bool {
    is_identifier_start(character) || character.is_ascii_digit()
}

fn normalize_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn current_char(contents: &str, index: usize) -> Option<char> {
    contents.get(index..)?.chars().next()
}

fn peek_char(contents: &str, index: usize) -> Option<char> {
    current_char(contents, index)
}

fn previous_char(contents: &str, start_index: usize) -> Option<char> {
    contents[..start_index].chars().next_back()
}

fn advance_one(contents: &str, index: usize) -> usize {
    current_char(contents, index)
        .map(|character| index + character.len_utf8())
        .unwrap_or(contents.len())
}

fn extension(path: &Path) -> &str {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| {
            if value.is_empty() {
                ""
            } else {
                match value {
                    "js" => ".js",
                    "jsx" => ".jsx",
                    "ts" => ".ts",
                    "tsx" => ".tsx",
                    "cjs" => ".cjs",
                    "cjsx" => ".cjsx",
                    "cts" => ".cts",
                    "ctsx" => ".ctsx",
                    "mjs" => ".mjs",
                    "mjsx" => ".mjsx",
                    "mts" => ".mts",
                    "mtsx" => ".mtsx",
                    "vue" => ".vue",
                    "svelte" => ".svelte",
                    _ => "",
                }
            }
        })
        .unwrap_or("")
}

fn to_posix_relative(project_root: &Path, file_path: &Path) -> String {
    file_path
        .strip_prefix(project_root)
        .unwrap_or(file_path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::merge_tree_shaking_warnings;
    use crate::{
        FindingAnalysisSource, FindingConfidence, FindingEvidence, FindingMetadata,
        TreeShakingWarning,
    };

    #[test]
    fn merge_tree_shaking_warnings_preserves_and_dedupes_finding_metadata() {
        let warnings = vec![
            TreeShakingWarning {
                key: "lodash-root-import".to_string(),
                package_name: "lodash".to_string(),
                message: "Root imports can keep extra code.".to_string(),
                recommendation: "Prefer per-method imports.".to_string(),
                estimated_kb: 20,
                files: vec!["src/App.tsx".to_string()],
                finding: FindingMetadata::new(
                    "tree-shaking:lodash-root-import",
                    FindingAnalysisSource::SourceImport,
                )
                .with_confidence(FindingConfidence::Medium)
                .with_evidence([FindingEvidence::new("source-file")
                    .with_file("src/App.tsx")
                    .with_specifier("lodash")]),
            },
            TreeShakingWarning {
                key: "lodash-root-import".to_string(),
                package_name: "lodash".to_string(),
                message: "Root imports can keep extra code.".to_string(),
                recommendation: "Prefer per-method imports.".to_string(),
                estimated_kb: 26,
                files: vec!["src/Dashboard.tsx".to_string()],
                finding: FindingMetadata::new(
                    "tree-shaking:lodash-root-import",
                    FindingAnalysisSource::SourceImport,
                )
                .with_confidence(FindingConfidence::High)
                .with_evidence([
                    FindingEvidence::new("source-file")
                        .with_file("src/Dashboard.tsx")
                        .with_specifier("lodash"),
                    FindingEvidence::new("source-file")
                        .with_file("src/App.tsx")
                        .with_specifier("lodash"),
                ]),
            },
        ];

        let merged = merge_tree_shaking_warnings(warnings);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].estimated_kb, 26);
        assert_eq!(
            merged[0].files,
            vec!["src/App.tsx".to_string(), "src/Dashboard.tsx".to_string()]
        );
        assert_eq!(
            merged[0].finding,
            FindingMetadata::new(
                "tree-shaking:lodash-root-import",
                FindingAnalysisSource::SourceImport,
            )
            .with_confidence(FindingConfidence::High)
            .with_evidence([
                FindingEvidence::new("source-file")
                    .with_file("src/App.tsx")
                    .with_specifier("lodash"),
                FindingEvidence::new("source-file")
                    .with_file("src/Dashboard.tsx")
                    .with_specifier("lodash"),
            ])
        );
    }
}
