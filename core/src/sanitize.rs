//! Sanitization module for MacAgentWatch
//!
//! Provides utilities to mask sensitive information in command arguments
//! such as passwords, API keys, and authentication tokens.

use std::borrow::Cow;
use std::sync::LazyLock;

/// Mask placeholder for sensitive data
const MASK: &str = "***";

/// Flags that indicate the next argument contains sensitive data
const SENSITIVE_FLAGS: &[&str] = &[
    "-p",
    "--password",
    "--token",
    "--api-key",
    "--apikey",
    "--secret",
    "-k",
    "--key",
    "--auth",
    "--auth-token",
    "--access-token",
    "--private-key",
];

/// Flags that contain sensitive data inline (--flag=value format)
const SENSITIVE_INLINE_FLAGS: &[&str] = &[
    "--password=",
    "--token=",
    "--api-key=",
    "--apikey=",
    "--secret=",
    "--key=",
    "--auth=",
    "--auth-token=",
    "--access-token=",
    "--private-key=",
];

/// Environment variable prefixes that contain sensitive data
const SENSITIVE_ENV_PREFIXES: &[&str] = &[
    "ANTHROPIC_API_KEY=",
    "OPENAI_API_KEY=",
    "AWS_SECRET_ACCESS_KEY=",
    "AWS_SESSION_TOKEN=",
    "GITHUB_TOKEN=",
    "GH_TOKEN=",
    "NPM_TOKEN=",
    "DOCKER_PASSWORD=",
    "DATABASE_PASSWORD=",
    "DB_PASSWORD=",
    "MYSQL_PASSWORD=",
    "POSTGRES_PASSWORD=",
    "REDIS_PASSWORD=",
    "SECRET_KEY=",
    "PRIVATE_KEY=",
    "API_KEY=",
    "API_SECRET=",
    "AUTH_TOKEN=",
    "ACCESS_TOKEN=",
    "REFRESH_TOKEN=",
];

/// Pre-computed lowercase versions of sensitive flags
static SENSITIVE_FLAGS_LOWER: LazyLock<Vec<String>> =
    LazyLock::new(|| SENSITIVE_FLAGS.iter().map(|f| f.to_lowercase()).collect());

/// Pre-computed lowercase versions of sensitive inline flags
static SENSITIVE_INLINE_FLAGS_LOWER: LazyLock<Vec<String>> = LazyLock::new(|| {
    SENSITIVE_INLINE_FLAGS
        .iter()
        .map(|f| f.to_lowercase())
        .collect()
});

/// Pre-computed lowercase versions of sensitive env prefixes
static SENSITIVE_ENV_PREFIXES_LOWER: LazyLock<Vec<String>> = LazyLock::new(|| {
    SENSITIVE_ENV_PREFIXES
        .iter()
        .map(|p| p.to_lowercase())
        .collect()
});

/// Sanitize command arguments by masking sensitive information
///
/// # Arguments
/// * `args` - The command arguments to sanitize
///
/// # Returns
/// A new vector with sensitive arguments masked
///
/// # Examples
/// ```
/// use macagentwatch_core::sanitize::sanitize_args;
///
/// let args = vec!["-p".to_string(), "secret123".to_string()];
/// let sanitized = sanitize_args(&args);
/// assert_eq!(sanitized, vec!["-p", "***"]);
/// ```
pub fn sanitize_args(args: &[String]) -> Vec<String> {
    let mut result = Vec::with_capacity(args.len());
    let mut mask_next = false;

    for arg in args {
        if mask_next {
            result.push(MASK.to_string());
            mask_next = false;
            continue;
        }

        // Check for flags that indicate next arg is sensitive (case-insensitive)
        let arg_lower = arg.to_lowercase();
        if SENSITIVE_FLAGS_LOWER.contains(&arg_lower) {
            result.push(arg.clone());
            mask_next = true;
            continue;
        }

        // Check for inline flag=value patterns
        if let Some(masked) = mask_inline_flag(arg) {
            result.push(masked);
            continue;
        }

        // Check for environment variable patterns
        if let Some(masked) = mask_env_variable(arg) {
            result.push(masked);
            continue;
        }

        // Check for token patterns in values
        if let Some(masked) = mask_token_patterns(arg) {
            result.push(masked);
            continue;
        }

        // Check for HTTP header patterns
        if let Some(masked) = mask_http_header(arg) {
            result.push(masked);
            continue;
        }

        // Check for URL with embedded credentials
        if let Some(masked) = mask_url_credentials(arg) {
            result.push(masked);
            continue;
        }

        result.push(arg.clone());
    }

    result
}

/// Mask inline flag=value patterns
fn mask_inline_flag(arg: &str) -> Option<String> {
    let arg_lower = arg.to_lowercase();
    for (prefix_lower, _original) in SENSITIVE_INLINE_FLAGS_LOWER
        .iter()
        .zip(SENSITIVE_INLINE_FLAGS.iter())
    {
        if arg_lower.starts_with(prefix_lower.as_str()) {
            if let Some(eq_pos) = arg.find('=') {
                let flag_part = &arg[..eq_pos];
                return Some(format!("{}={}", flag_part, MASK));
            }
        }
    }
    None
}

/// Mask environment variable patterns
fn mask_env_variable(arg: &str) -> Option<String> {
    let arg_lower = arg.to_lowercase();
    for prefix_lower in SENSITIVE_ENV_PREFIXES_LOWER.iter() {
        if arg_lower.starts_with(prefix_lower.as_str()) {
            if let Some(eq_pos) = arg.find('=') {
                let var_name = &arg[..eq_pos];
                return Some(format!("{}={}", var_name, MASK));
            }
        }
    }
    None
}

/// Mask common token patterns in argument values
fn mask_token_patterns(arg: &str) -> Option<String> {
    // Anthropic API key: sk-ant-api03-...
    if arg.starts_with("sk-ant-") {
        return Some(format!("sk-ant-{}", MASK));
    }

    // OpenAI API key: sk-... (longer than 20 chars to avoid false positives)
    if arg.starts_with("sk-") && arg.len() > 20 && !arg.starts_with("sk-ant-") {
        return Some(format!("sk-{}", MASK));
    }

    // GitHub token: ghp_... or gho_... or ghs_... or ghr_...
    if arg.starts_with("ghp_")
        || arg.starts_with("gho_")
        || arg.starts_with("ghs_")
        || arg.starts_with("ghr_")
    {
        let prefix = &arg[..4];
        return Some(format!("{}{}", prefix, MASK));
    }

    // AWS access key: AKIA... or ASIA...
    if (arg.starts_with("AKIA") || arg.starts_with("ASIA")) && arg.len() == 20 {
        return Some(MASK.to_string());
    }

    // npm token: npm_...
    if arg.starts_with("npm_") {
        return Some(format!("npm_{}", MASK));
    }

    None
}

/// Mask sensitive HTTP header values
fn mask_http_header(arg: &str) -> Option<String> {
    let arg_lower = arg.to_lowercase();

    // Bearer token in Authorization header
    if arg_lower.starts_with("bearer ") {
        return Some(format!("Bearer {}", MASK));
    }

    // Basic auth in Authorization header
    if arg_lower.starts_with("basic ") {
        return Some(format!("Basic {}", MASK));
    }

    // Full Authorization header format
    if arg_lower.starts_with("authorization:") {
        if let Some(colon_pos) = arg.find(':') {
            let header_name = &arg[..colon_pos];
            return Some(format!("{}: {}", header_name, MASK));
        }
    }

    // X-Api-Key header
    if arg_lower.starts_with("x-api-key:") {
        if let Some(colon_pos) = arg.find(':') {
            let header_name = &arg[..colon_pos];
            return Some(format!("{}: {}", header_name, MASK));
        }
    }

    None
}

/// Mask credentials embedded in URLs (e.g., https://user:password@host.com)
fn mask_url_credentials(arg: &str) -> Option<String> {
    // Match patterns like scheme://user:pass@host
    if let Some(scheme_end) = arg.find("://") {
        let after_scheme = &arg[scheme_end + 3..];
        if let Some(at_pos) = after_scheme.find('@') {
            let credentials = &after_scheme[..at_pos];
            if credentials.contains(':') {
                let scheme = &arg[..scheme_end + 3];
                let host_part = &after_scheme[at_pos + 1..];
                return Some(format!("{}{}@{}", scheme, MASK, host_part));
            }
        }
    }
    None
}

/// Sanitize a full command string (command + args joined)
///
/// This is useful for sanitizing commands that are passed as a single string.
/// Handles basic quoting (single and double quotes).
pub fn sanitize_command_string(command: &str) -> Cow<'_, str> {
    // Check for common patterns that need sanitization
    let command_lower = command.to_lowercase();
    let needs_sanitization = SENSITIVE_FLAGS_LOWER
        .iter()
        .any(|f| command_lower.contains(f.as_str()))
        || command.contains("sk-ant-")
        || command.contains("sk-")
        || command.contains("ghp_")
        || command.contains("Bearer ")
        || command.contains("://")
        || SENSITIVE_ENV_PREFIXES_LOWER
            .iter()
            .any(|p| command_lower.contains(p.as_str()));

    if !needs_sanitization {
        return Cow::Borrowed(command);
    }

    // Shell-aware splitting that respects quotes
    let parts = shell_split(command);

    Cow::Owned(sanitize_args(&parts).join(" "))
}

/// Simple shell-aware string splitting that respects single and double quotes
fn shell_split(input: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
                // Don't include the quote character in the token
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
                // Don't include the quote character in the token
            }
            '\\' if in_double_quote || !in_single_quote => {
                // Escaped character — include the next char literally
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            ' ' | '\t' if !in_single_quote && !in_double_quote => {
                if !current.is_empty() {
                    parts.push(std::mem::take(&mut current));
                }
            }
            _ => {
                current.push(c);
            }
        }
    }

    if !current.is_empty() {
        parts.push(current);
    }

    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_flag() {
        let args = vec!["-p".to_string(), "secret123".to_string()];
        let result = sanitize_args(&args);
        assert_eq!(result, vec!["-p", "***"]);
    }

    #[test]
    fn test_password_long_flag() {
        let args = vec!["--password".to_string(), "mysecret".to_string()];
        let result = sanitize_args(&args);
        assert_eq!(result, vec!["--password", "***"]);
    }

    #[test]
    fn test_inline_password() {
        let args = vec!["--password=secret123".to_string()];
        let result = sanitize_args(&args);
        assert_eq!(result, vec!["--password=***"]);
    }

    #[test]
    fn test_token_flag() {
        let args = vec!["--token".to_string(), "abc123".to_string()];
        let result = sanitize_args(&args);
        assert_eq!(result, vec!["--token", "***"]);
    }

    #[test]
    fn test_anthropic_api_key() {
        let args = vec!["sk-ant-api03-abcdef123456".to_string()];
        let result = sanitize_args(&args);
        assert_eq!(result, vec!["sk-ant-***"]);
    }

    #[test]
    fn test_openai_api_key() {
        let args = vec!["sk-abcdefghijklmnopqrstuvwxyz".to_string()];
        let result = sanitize_args(&args);
        assert_eq!(result, vec!["sk-***"]);
    }

    #[test]
    fn test_github_token() {
        let args = vec!["ghp_abcdefghijklmnopqrstuvwxyz".to_string()];
        let result = sanitize_args(&args);
        assert_eq!(result, vec!["ghp_***"]);
    }

    #[test]
    fn test_bearer_token() {
        let args = vec![
            "Bearer".to_string(),
            "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9".to_string(),
        ];
        let result = sanitize_args(&args);
        // Bearer is not a sensitive flag, but the next token might be masked by token patterns
        assert_eq!(result[0], "Bearer");
    }

    #[test]
    fn test_bearer_header() {
        let args = vec!["Bearer sk-ant-api03-test".to_string()];
        let result = sanitize_args(&args);
        assert_eq!(result, vec!["Bearer ***"]);
    }

    #[test]
    fn test_env_variable() {
        let args = vec!["ANTHROPIC_API_KEY=sk-ant-api03-test".to_string()];
        let result = sanitize_args(&args);
        assert_eq!(result, vec!["ANTHROPIC_API_KEY=***"]);
    }

    #[test]
    fn test_aws_secret() {
        let args =
            vec!["AWS_SECRET_ACCESS_KEY=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string()];
        let result = sanitize_args(&args);
        assert_eq!(result, vec!["AWS_SECRET_ACCESS_KEY=***"]);
    }

    #[test]
    fn test_mixed_args() {
        let args = vec![
            "curl".to_string(),
            "-H".to_string(),
            "Authorization: Bearer sk-ant-test".to_string(),
            "https://api.example.com".to_string(),
        ];
        let result = sanitize_args(&args);
        assert_eq!(result[0], "curl");
        assert_eq!(result[1], "-H");
        assert_eq!(result[2], "Authorization: ***");
        assert_eq!(result[3], "https://api.example.com");
    }

    #[test]
    fn test_no_sensitive_data() {
        let args = vec!["ls".to_string(), "-la".to_string(), "/home".to_string()];
        let result = sanitize_args(&args);
        assert_eq!(result, args);
    }

    #[test]
    fn test_multiple_sensitive_flags() {
        let args = vec![
            "--password".to_string(),
            "pass1".to_string(),
            "--token".to_string(),
            "tok1".to_string(),
        ];
        let result = sanitize_args(&args);
        assert_eq!(result, vec!["--password", "***", "--token", "***"]);
    }

    #[test]
    fn test_sanitize_command_string() {
        let cmd = "curl -H Bearer sk-ant-test https://api.example.com";
        let result = sanitize_command_string(cmd);
        assert!(result.contains("***"));
        assert!(!result.contains("sk-ant-test"));
    }

    #[test]
    fn test_sanitize_command_string_no_sensitive() {
        let cmd = "ls -la /home";
        let result = sanitize_command_string(cmd);
        assert_eq!(result.as_ref(), cmd);
    }

    #[test]
    fn test_case_insensitive_inline_flag() {
        let args = vec!["--PASSWORD=secret".to_string()];
        let result = sanitize_args(&args);
        assert_eq!(result, vec!["--PASSWORD=***"]);
    }

    #[test]
    fn test_npm_token() {
        let args = vec!["npm_abcdefghijklmnop".to_string()];
        let result = sanitize_args(&args);
        assert_eq!(result, vec!["npm_***"]);
    }

    #[test]
    fn test_x_api_key_header() {
        let args = vec!["X-Api-Key: sk-ant-test123".to_string()];
        let result = sanitize_args(&args);
        assert_eq!(result, vec!["X-Api-Key: ***"]);
    }

    #[test]
    fn test_basic_auth_header() {
        let args = vec!["Basic dXNlcjpwYXNzd29yZA==".to_string()];
        let result = sanitize_args(&args);
        assert_eq!(result, vec!["Basic ***"]);
    }

    #[test]
    fn test_case_insensitive_flag() {
        let args = vec!["--Password".to_string(), "secret".to_string()];
        let result = sanitize_args(&args);
        assert_eq!(result, vec!["--Password", "***"]);
    }

    #[test]
    fn test_case_insensitive_flag_uppercase() {
        let args = vec!["--TOKEN".to_string(), "abc123".to_string()];
        let result = sanitize_args(&args);
        assert_eq!(result, vec!["--TOKEN", "***"]);
    }

    #[test]
    fn test_case_insensitive_env_variable() {
        let args = vec!["anthropic_api_key=sk-ant-test".to_string()];
        let result = sanitize_args(&args);
        assert_eq!(result, vec!["anthropic_api_key=***"]);
    }

    #[test]
    fn test_url_credentials() {
        let args = vec!["https://admin:password123@db.example.com/mydb".to_string()];
        let result = sanitize_args(&args);
        assert_eq!(result, vec!["https://***@db.example.com/mydb"]);
    }

    #[test]
    fn test_url_no_credentials() {
        let args = vec!["https://example.com/path".to_string()];
        let result = sanitize_args(&args);
        assert_eq!(result, vec!["https://example.com/path"]);
    }

    #[test]
    fn test_quoted_command_string() {
        let cmd = r#"curl --password="my secret" https://api.example.com"#;
        let result = sanitize_command_string(cmd);
        assert!(result.contains("***"));
        assert!(!result.contains("my secret"));
    }

    #[test]
    fn test_shell_split_basic() {
        let parts = shell_split("hello world");
        assert_eq!(parts, vec!["hello", "world"]);
    }

    #[test]
    fn test_shell_split_double_quotes() {
        let parts = shell_split(r#"--password="my secret""#);
        assert_eq!(parts, vec!["--password=my secret"]);
    }

    #[test]
    fn test_shell_split_single_quotes() {
        let parts = shell_split("--token='abc def'");
        assert_eq!(parts, vec!["--token=abc def"]);
    }
}
