use std::path::{Path, PathBuf};
use crate::error::{ApiError, Result};

/// Validates that a path is within the workspace boundary
pub fn validate_path_in_workspace(file_path: &Path, workspace_path: &Path) -> Result<PathBuf> {
    // If the path exists, canonicalize it directly
    if file_path.exists() {
        let resolved = file_path.canonicalize()
            .map_err(|e| ApiError {
                code: "PATH_ERROR".to_string(),
                message: format!("Failed to resolve path: {}", e),
                details: None,
            })?;
        
        let workspace_resolved = workspace_path.canonicalize()
            .map_err(|e| ApiError {
                code: "WORKSPACE_ERROR".to_string(),
                message: format!("Failed to resolve workspace: {}", e),
                details: None,
            })?;
        
        if !resolved.starts_with(&workspace_resolved) {
            return Err(ApiError {
                code: "PATH_TRAVERSAL".to_string(),
                message: format!("Path traversal attempt detected: {:?} is outside workspace", file_path),
                details: None,
            });
        }
        
        return Ok(resolved);
    }

    // If the path doesn't exist, validate its parent directory
    let parent = file_path.parent().ok_or_else(|| ApiError {
        code: "PATH_ERROR".to_string(),
        message: "Path has no parent".to_string(),
        details: None,
    })?;

    // Recurse to find the first existing parent
    let mut current = parent;
    while !current.exists() {
        if let Some(p) = current.parent() {
            current = p;
        } else {
            break;
        }
    }

    let resolved_parent = current.canonicalize()
        .map_err(|e| ApiError {
            code: "PATH_ERROR".to_string(),
            message: format!("Failed to resolve parent path: {}", e),
            details: None,
        })?;

    let workspace_resolved = workspace_path.canonicalize()
        .map_err(|e| ApiError {
            code: "WORKSPACE_ERROR".to_string(),
            message: format!("Failed to resolve workspace: {}", e),
            details: None,
        })?;

    if !resolved_parent.starts_with(&workspace_resolved) {
        return Err(ApiError {
            code: "PATH_TRAVERSAL".to_string(),
            message: format!("Path traversal attempt detected: {:?} is outside workspace", file_path),
            details: None,
        });
    }

    // Return the joined path (not canonicalized because it doesn't exist yet)
    Ok(file_path.to_path_buf())
}

/// Checks if a file is likely binary by reading first 1024 bytes
pub async fn is_binary_file(path: &Path) -> Result<bool> {
    let content = tokio::fs::read(path).await?;
    
    // Check for null bytes (common in binary files)
    Ok(content.iter().any(|&b| b == 0))
}

/// Gets file extension
pub fn get_file_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase())
}

/// Checks if file should be skipped (binary, node_modules, etc.)
pub fn should_skip_file(path: &Path) -> bool {
    let skip_dirs = [
        "node_modules", ".git", "dist", "dist-electron", ".next",
        "__pycache__", ".venv", "venv", ".cache", "coverage",
        ".idea", ".vscode", "build", "out", "bin", "obj",
    ];
    
    let binary_exts = [
        "png", "jpg", "jpeg", "gif", "ico", "webp", "svg",
        "woff", "woff2", "ttf", "eot", "mp3", "mp4", "zip",
        "tar", "gz", "exe", "dll", "so", "dylib", "lock",
        "pdf", "bin", "pyc", "node",
    ];
    
    // Check if any parent directory should be skipped
    for component in path.components() {
        if let std::path::Component::Normal(name) = component {
            if let Some(name_str) = name.to_str() {
                if skip_dirs.contains(&name_str) {
                    return true;
                }
            }
        }
    }
    
    // Check file extension
    if let Some(ext) = get_file_extension(path) {
        if binary_exts.contains(&ext.as_str()) {
            return true;
        }
    }
    
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_skip_file() {
        assert!(should_skip_file(Path::new("node_modules/package/index.js")));
        assert!(should_skip_file(Path::new("dist/bundle.js")));
        assert!(should_skip_file(Path::new("image.png")));
        assert!(!should_skip_file(Path::new("src/main.ts")));
    }
}
