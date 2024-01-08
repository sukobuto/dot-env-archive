use std::path::{Path, PathBuf};

pub fn search_env_files(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let files = globmatch::Builder::new("**/{.env,.env.*}")
        .build(dir)
        .expect("Failed to build globmatch")
        .into_iter()
        .filter_entry(|entry| {
            entry
                .components()
                .all(|component| component.as_os_str() != "node_modules")
        })
        .flatten()
        .collect::<Vec<_>>();
    Ok(files)
}

#[cfg(test)]
mod tests_search_env_files {
    use super::*;

    #[test]
    fn 期待するファイルがヒットする() {
        let tmp_dir = tempfile::tempdir().unwrap();
        std::fs::File::create(tmp_dir.path().join(".env")).unwrap();
        std::fs::File::create(tmp_dir.path().join(".env.local")).unwrap();
        let files = search_env_files(tmp_dir.path()).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn node_modulesディレクトリが除外される() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let node_modules_dir = tmp_dir.path().join("node_modules");
        let env_file = node_modules_dir.join(".env");
        std::fs::create_dir(node_modules_dir).unwrap();
        std::fs::File::create(env_file).unwrap();
        let files = search_env_files(tmp_dir.path()).unwrap();
        assert_eq!(files.len(), 0);
    }
}
