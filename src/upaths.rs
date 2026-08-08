use std::env;
use std::path::{Path, PathBuf};

pub fn expand_path(raw: &str) -> PathBuf {
    let with_vars = expand_env_vars(raw);
    let with_home = expand_home(&with_vars);
    let path = Path::new(&with_home);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(path)
    }
}

fn expand_home(input: &str) -> String {
    if let Some(rest) = input.strip_prefix('~') {
        if rest.is_empty() || rest.starts_with('/') {
            if let Some(home) = env::var_os("HOME") {
                let home = home.to_string_lossy().into_owned();
                return format!("{home}{rest}");
            }
        }
    }
    input.to_string()
}

fn expand_env_vars(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '$' && i + 1 < chars.len() {
            if chars[i + 1] == '{' {
                if let Some(end) = chars[i + 2..].iter().position(|&x| x == '}') {
                    let name: String = chars[i + 2..i + 2 + end].iter().collect();
                    if let Ok(value) = env::var(&name) {
                        result.push_str(&value);
                    }
                    i = i + 2 + end + 1;
                    continue;
                }
            } else if chars[i + 1].is_alphabetic() || chars[i + 1] == '_' {
                let start = i + 1;
                let mut end = start;
                while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
                    end += 1;
                }
                let name: String = chars[start..end].iter().collect();
                if let Ok(value) = env::var(&name) {
                    result.push_str(&value);
                }
                i = end;
                continue;
            }
        }
        result.push(c);
        i += 1;
    }
    result
}

pub fn truncate_for_display(path: &Path) -> String {
    let components: Vec<String> = path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    if components.len() <= 3 {
        return path.to_string_lossy().into_owned();
    }
    let tail = &components[components.len() - 3..];
    format!(".../{}", tail.join("/"))
}
