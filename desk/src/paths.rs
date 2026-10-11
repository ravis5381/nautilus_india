use std::path::{Path, PathBuf};

pub fn web_dist() -> PathBuf {
    if let Ok(value) = std::env::var("DESK_WEB") {
        let value = value.trim();
        if !value.is_empty() {
            return resolve_web(value, env_file().as_deref().and_then(Path::parent));
        }
    }
    beside_executable("web/dist")
        .filter(|path| path.join("index.html").is_file())
        .unwrap_or_else(|| manifest_dir().join("web/dist"))
}

fn resolve_web(value: &str, env_dir: Option<&Path>) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        return path;
    }
    match env_dir {
        Some(dir) => dir.join(path),
        None => path,
    }
}

pub fn env_file() -> Option<PathBuf> {
    beside_executable(".env")
        .filter(|path| path.is_file())
        .or_else(|| {
            let dev = manifest_dir().join("../.env");
            dev.is_file().then_some(dev)
        })
}

pub fn default_database() -> PathBuf {
    if beside_executable("web/dist/index.html").is_some_and(|path| path.is_file()) {
        return beside_executable("data/positions.db").unwrap_or_else(manifest_database);
    }
    manifest_database()
}

fn manifest_database() -> PathBuf {
    manifest_dir().join("data").join("positions.db")
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn beside_executable(relative: impl AsRef<Path>) -> Option<PathBuf> {
    let executable = std::env::current_exe().ok()?;
    let executable = executable.canonicalize().unwrap_or(executable);
    Some(executable.parent()?.join(relative))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_relative_frontend_path_sits_beside_the_env_file() {
        let path = resolve_web("desk/web/dist", Some(Path::new("/opt/desk")));
        assert_eq!(path, PathBuf::from("/opt/desk/desk/web/dist"));
    }

    #[test]
    fn an_absolute_frontend_path_is_used_as_written() {
        let path = resolve_web("/var/www/desk", Some(Path::new("/opt/desk")));
        assert_eq!(path, PathBuf::from("/var/www/desk"));
    }
}
