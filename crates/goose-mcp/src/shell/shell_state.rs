use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Manages persistent shell state across command invocations
#[derive(Debug, Clone)]
pub struct ShellState {
    working_dir: PathBuf,
    env_vars: HashMap<String, String>,
}

impl ShellState {
    pub fn new() -> Self {
        let working_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            working_dir,
            env_vars: HashMap::new(),
        }
    }

    /// Parse a command and extract state changes (cd, export)
    /// Returns the modified command with state restoration prepended
    pub fn wrap_command(&mut self, command: &str) -> String {
        // First, extract any cd or export commands from the user's command
        self.extract_state_changes(command);

        // Build the state restoration prefix
        let mut prefix_parts = Vec::new();

        // Add cd command if working dir has changed
        if self.working_dir != std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")) {
            prefix_parts.push(format!("cd {}", shell_escape(&self.working_dir)));
        }

        // Add export commands for tracked env vars
        for (key, value) in &self.env_vars {
            prefix_parts.push(format!("export {}={}", key, shell_escape_value(value)));
        }

        // Combine prefix with user command
        if prefix_parts.is_empty() {
            command.to_string()
        } else {
            format!("{} && {}", prefix_parts.join(" && "), command)
        }
    }

    /// Extract state changes from a command and update internal state
    fn extract_state_changes(&mut self, command: &str) {
        // Simple pattern matching for cd commands
        // Handles: cd <path>, cd, cd ~, cd -, etc.
        if let Some(cd_path) = Self::extract_cd_path(command) {
            self.update_working_dir(&cd_path);
        }

        // Extract export commands
        // Handles: export VAR=value, export VAR="value", export VAR='value'
        for (key, value) in Self::extract_exports(command) {
            self.env_vars.insert(key, value);
        }
    }

    /// Extract cd path from command if present
    fn extract_cd_path(command: &str) -> Option<String> {
        // Look for cd commands - this is a simple heuristic
        // We look for patterns like: cd path, cd "path", cd 'path'
        let cd_pattern =
            regex::Regex::new(r#"(?:^|[;&|])\s*cd(?:\s+([^\s;&|'"]+|"[^"]+"|'[^']+'))?"#).ok()?;

        cd_pattern.captures(command).and_then(|caps| {
            caps.get(1).map(|m| {
                let path = m.as_str();
                // Remove quotes if present
                if (path.starts_with('"') && path.ends_with('"'))
                    || (path.starts_with('\'') && path.ends_with('\''))
                {
                    path.strip_prefix(|c| c == '"' || c == '\'')
                        .and_then(|s| s.strip_suffix(|c| c == '"' || c == '\''))
                        .unwrap_or(path)
                        .to_string()
                } else {
                    path.to_string()
                }
            })
        })
    }

    /// Extract export statements from command
    fn extract_exports(command: &str) -> Vec<(String, String)> {
        let mut exports = Vec::new();

        // Match export VAR=value patterns
        let export_pattern =
            regex::Regex::new(r#"export\s+([A-Za-z_][A-Za-z0-9_]*)=([^\s;&|'"]+|"[^"]*"|'[^']*')"#)
                .unwrap();

        for caps in export_pattern.captures_iter(command) {
            if let (Some(key), Some(value)) = (caps.get(1), caps.get(2)) {
                let key = key.as_str().to_string();
                let value_str = value.as_str();

                // Remove quotes if present
                let value = if (value_str.starts_with('"') && value_str.ends_with('"'))
                    || (value_str.starts_with('\'') && value_str.ends_with('\''))
                {
                    value_str
                        .strip_prefix(|c| c == '"' || c == '\'')
                        .and_then(|s| s.strip_suffix(|c| c == '"' || c == '\''))
                        .unwrap_or(value_str)
                        .to_string()
                } else {
                    value_str.to_string()
                };

                exports.push((key, value));
            }
        }

        exports
    }

    /// Update the working directory based on a cd path
    fn update_working_dir(&mut self, path: &str) {
        let new_dir = if path.is_empty() || path == "~" {
            // cd with no args or cd ~ goes to home
            dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
        } else if path == "-" {
            // cd - would go to previous dir, but we don't track that yet
            // For now, just keep current dir
            return;
        } else if path.starts_with('/') {
            // Absolute path
            PathBuf::from(path)
        } else if let Some(stripped) = path.strip_prefix("~/") {
            // Home-relative path
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(stripped)
        } else {
            // Relative path
            self.working_dir.join(path)
        };

        // Normalize the path (resolve .., ., etc.)
        if let Ok(canonical) = new_dir.canonicalize() {
            self.working_dir = canonical;
            // If canonicalize fails (directory doesn't exist), don't update state
            // This prevents failed cd commands from corrupting the persistent state
        }
    }

    #[allow(dead_code)]
    pub fn working_dir(&self) -> &PathBuf {
        &self.working_dir
    }

    #[allow(dead_code)]
    pub fn env_vars(&self) -> &HashMap<String, String> {
        &self.env_vars
    }
}

impl Default for ShellState {
    fn default() -> Self {
        Self::new()
    }
}

/// Escape a path for use in shell commands
fn shell_escape(path: &Path) -> String {
    let path_str = path.to_string_lossy();
    if path_str.contains(' ') || path_str.contains('\'') || path_str.contains('"') {
        format!("'{}'", path_str.replace('\'', r"'\''"))
    } else {
        path_str.to_string()
    }
}

/// Escape a value for use in shell export commands
fn shell_escape_value(value: &str) -> String {
    if value.contains(' ')
        || value.contains('\'')
        || value.contains('"')
        || value.contains('$')
        || value.contains('`')
    {
        format!("'{}'", value.replace('\'', r"'\''"))
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_cd_path() {
        assert_eq!(
            ShellState::extract_cd_path("cd /tmp"),
            Some("/tmp".to_string())
        );
        assert_eq!(
            ShellState::extract_cd_path("cd /tmp && ls"),
            Some("/tmp".to_string())
        );
        assert_eq!(
            ShellState::extract_cd_path("ls && cd /tmp"),
            Some("/tmp".to_string())
        );
        assert_eq!(
            ShellState::extract_cd_path("cd '/tmp/my dir'"),
            Some("/tmp/my dir".to_string())
        );
        assert_eq!(ShellState::extract_cd_path("ls"), None);
    }

    #[test]
    fn test_extract_exports() {
        let exports = ShellState::extract_exports("export FOO=bar");
        assert_eq!(exports, vec![("FOO".to_string(), "bar".to_string())]);

        let exports = ShellState::extract_exports("export FOO=bar && export BAZ=qux");
        assert_eq!(
            exports,
            vec![
                ("FOO".to_string(), "bar".to_string()),
                ("BAZ".to_string(), "qux".to_string())
            ]
        );

        let exports = ShellState::extract_exports("export FOO='bar baz'");
        assert_eq!(exports, vec![("FOO".to_string(), "bar baz".to_string())]);

        let exports = ShellState::extract_exports("ls");
        assert_eq!(exports, Vec::<(String, String)>::new());
    }

    #[test]
    fn test_shell_state_persistence() {
        let mut state = ShellState::new();

        // Simulate cd command
        let wrapped = state.wrap_command("cd /tmp && ls");
        assert!(wrapped.contains("ls"));

        // After processing, state should be updated
        // On macOS, /tmp is a symlink to /private/tmp, so canonicalize
        let expected_path = std::fs::canonicalize("/tmp").unwrap_or_else(|_| PathBuf::from("/tmp"));
        assert_eq!(state.working_dir(), &expected_path);

        // Next command should restore to /tmp (or /private/tmp on macOS)
        let wrapped = state.wrap_command("pwd");
        assert!(wrapped.contains("cd"));
        assert!(wrapped.contains("pwd"));
    }

    #[test]
    fn test_export_persistence() {
        let mut state = ShellState::new();

        // Simulate export command
        let wrapped = state.wrap_command("export FOO=bar && echo $FOO");
        assert!(wrapped.contains("echo $FOO"));

        // Check state was updated
        assert_eq!(state.env_vars().get("FOO"), Some(&"bar".to_string()));

        // Next command should restore FOO
        let wrapped = state.wrap_command("echo $FOO");
        assert!(wrapped.contains("export FOO=bar"));
        assert!(wrapped.contains("echo $FOO"));
    }

    #[test]
    fn test_combined_state() {
        let mut state = ShellState::new();

        // Set both cd and export
        let wrapped = state.wrap_command("cd /tmp && export FOO=bar && ls");
        assert!(wrapped.contains("ls"));

        // Next command should restore both
        let wrapped = state.wrap_command("pwd");
        // Should contain cd command (path might be /tmp or /private/tmp on macOS)
        assert!(wrapped.contains("cd"));
        assert!(wrapped.contains("export FOO=bar"));
        assert!(wrapped.contains("pwd"));
    }

    #[test]
    fn test_shell_escape() {
        assert_eq!(shell_escape(&PathBuf::from("/tmp/my dir")), "'/tmp/my dir'");
        assert_eq!(shell_escape(&PathBuf::from("/tmp/simple")), "/tmp/simple");
        assert_eq!(shell_escape(&PathBuf::from("/tmp/it's")), "'/tmp/it'\\''s'");
    }

    #[test]
    fn test_shell_escape_value() {
        assert_eq!(shell_escape_value("simple"), "simple");
        assert_eq!(shell_escape_value("with space"), "'with space'");
        assert_eq!(shell_escape_value("with'quote"), "'with'\\''quote'");
        assert_eq!(shell_escape_value("with$dollar"), "'with$dollar'");
    }
}

#[test]
fn test_failed_cd_does_not_corrupt_state() {
    let mut state = ShellState::new();
    let original_dir = state.working_dir().clone();

    // Try to cd to a non-existent directory
    let wrapped = state.wrap_command("cd /this/directory/definitely/does/not/exist/12345");

    // State should not have changed since directory does not exist
    assert_eq!(
        state.working_dir(),
        &original_dir,
        "Working directory should not change for non-existent path"
    );

    // Verify the wrapped command still includes the cd attempt
    assert!(wrapped.contains("cd /this/directory/definitely/does/not/exist/12345"));

    // Now do a valid cd and verify it works
    let _wrapped = state.wrap_command("cd /tmp");
    assert!(state.working_dir().to_string_lossy().contains("tmp"));
}
