use std::{collections::HashSet, path::Path};

use serde_json::Value;

use crate::{error::Result, workspace::exists};

struct FrameworkMarker {
    name: &'static str,
    packages: &'static [&'static str],
    files: &'static [&'static str],
}

const FRAMEWORK_MARKERS: [FrameworkMarker; 9] = [
    FrameworkMarker {
        name: "Next.js",
        packages: &["next"],
        files: &["next.config.js", "next.config.mjs", "next.config.ts"],
    },
    FrameworkMarker {
        name: "Vite",
        packages: &["vite"],
        files: &["vite.config.js", "vite.config.ts", "vite.config.mjs"],
    },
    FrameworkMarker {
        name: "Webpack",
        packages: &["webpack"],
        files: &["webpack.config.js", "webpack.config.ts"],
    },
    FrameworkMarker {
        name: "Rollup",
        packages: &["rollup"],
        files: &["rollup.config.js", "rollup.config.mjs", "rollup.config.ts"],
    },
    FrameworkMarker {
        name: "Astro",
        packages: &["astro"],
        files: &["astro.config.mjs", "astro.config.ts"],
    },
    FrameworkMarker {
        name: "Nuxt",
        packages: &["nuxt"],
        files: &["nuxt.config.ts", "nuxt.config.js"],
    },
    FrameworkMarker {
        name: "React",
        packages: &["react"],
        files: &[],
    },
    FrameworkMarker {
        name: "Vue",
        packages: &["vue"],
        files: &[],
    },
    FrameworkMarker {
        name: "Svelte",
        packages: &["svelte", "@sveltejs/kit"],
        files: &[],
    },
];

const PACKAGE_MANAGER_CHECKS: [(&str, &str); 5] = [
    ("pnpm-lock.yaml", "pnpm"),
    ("yarn.lock", "yarn"),
    ("package-lock.json", "npm"),
    ("bun.lockb", "bun"),
    ("bun.lock", "bun"),
];

pub fn detect_frameworks(project_root: &Path, manifest: &Value) -> Result<Vec<String>> {
    let all_dependencies = dependency_names(manifest);
    let mut detected = Vec::new();

    for marker in FRAMEWORK_MARKERS {
        let package_hit = marker
            .packages
            .iter()
            .any(|package| all_dependencies.contains(*package));
        let file_hit = any_exists(project_root, marker.files)?;

        if package_hit || file_hit {
            detected.push(marker.name.to_string());
        }
    }

    Ok(detected)
}

pub fn detect_package_manager(project_root: &Path, manifest: &Value) -> Result<String> {
    if let Some(explicit) = manifest
        .get("packageManager")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
    {
        return Ok(explicit.to_string());
    }

    for (file, name) in PACKAGE_MANAGER_CHECKS {
        if exists(project_root.join(file))? {
            return Ok(name.to_string());
        }
    }

    Ok("unknown".to_string())
}

fn dependency_names(manifest: &Value) -> HashSet<String> {
    let mut dependencies = HashSet::new();

    for field in ["dependencies", "devDependencies"] {
        if let Some(entries) = manifest.get(field).and_then(Value::as_object) {
            dependencies.extend(entries.keys().cloned());
        }
    }

    dependencies
}

fn any_exists(project_root: &Path, files: &[&str]) -> Result<bool> {
    for file in files {
        if exists(project_root.join(file))? {
            return Ok(true);
        }
    }

    Ok(false)
}
