//! Sanitization module for MacAgentWatch
//!
//! Provides utilities to mask sensitive information in command arguments
//! such as passwords, API keys, and authentication tokens.

use std::borrow::Cow;

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

        // Check for flags that indicate next arg is sensitive
        if SENSITIVE_FLAGS.contains(&arg.as_str()) {
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

        result.push(arg.clone());
    }

    result
}

/// Mask inline flag=value patterns
fn mask_inline_flag(arg: &str) -> Option<String> {
    for prefix in SENSITIVE_INLINE_FLAGS {
        if arg.to_lowercase().starts_with(&prefix.to_lowercase()) {
            // Use find('=') for UTF-8 safe slicing
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
    for prefix in SENSITIVE_ENV_PREFIXES {
        if arg.starts_with(prefix) {
            // Use find('=') for UTF-8 safe slicing
            if let Some(eq_pos) = prefix.find('=') {
                let var_name = &prefix[..eq_pos];
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
    if arg.starts_with("ghp_") || arg.starts_with("gho_")
        || arg.starts_with("ghs_") || arg.starts_with("ghr_") {
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

/// Sanitize a full command string (command + args joined)
///
/// This is useful for sanitizing commands that are passed as a single string
pub fn sanitize_command_string(command: &str) -> Cow<'_, str> {
    // Check for common patterns that need sanitization
    let needs_sanitization = SENSITIVE_FLAGS.iter().any(|f| command.contains(f))
        || command.contains("sk-ant-")
        || command.contains("sk-")
        || command.contains("ghp_")
        || command.contains("Bearer ")
        || SENSITIVE_ENV_PREFIXES.iter().any(|p| command.contains(p));

    if !needs_sanitization {
        return Cow::Borrowed(command);
    }

    // Split and sanitize
    let parts: Vec<String> = command
        .split_whitespace()
        .map(String::from)
        .collect();

    Cow::Owned(sanitize_args(&parts).join(" "))
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
        let args = vec!["Bearer".to_string(), "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9".to_string()];
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
        let args = vec!["AWS_SECRET_ACCESS_KEY=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string()];
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
}
