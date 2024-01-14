use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};

pub struct Archive {
    database_path: PathBuf,
}

impl Archive {
    pub fn new(database_path: PathBuf) -> Self {
        Self { database_path }
    }

    /// データベースを初期化する
    pub async fn initialize(&self) -> anyhow::Result<()> {
        let query = r#"
            CREATE TABLE IF NOT EXISTS archives (
                name TEXT NOT NULL UNIQUE,
                path TEXT NOT NULL,
                created_at TEXT NOT NULL,
                body TEXT NOT NULL,
                checksum TEXT NOT NULL,
                PRIMARY KEY (path, created_at)
            );
            CREATE INDEX IF NOT EXISTS archives_path_idx ON archives (path);
            CREATE INDEX IF NOT EXISTS archives_created_at_idx ON archives (created_at);
        "#;
        let conn = Connection::open(&self.database_path)?;
        conn.execute_batch(query)?;

        Ok(())
    }

    /// env_file_path の内容が、最新のアーカイブと同じかどうかをチェックする
    pub async fn check_is_same_as_latest(&self, env_file_path: &Path) -> anyhow::Result<bool> {
        let checksum = crate::digest::file_checksum(env_file_path).await?;
        let conn = Connection::open(&self.database_path)?;
        let mut stmt = conn.prepare(
            "SELECT checksum FROM archives WHERE path = ?1 ORDER BY created_at DESC LIMIT 1",
        )?;
        let rows = stmt.query_map([env_file_path.to_string_lossy()], |row| {
            row.get::<_, String>(0)
        })?;

        if let Some(row) = rows.into_iter().next() {
            let row = row?;
            return Ok(row == checksum);
        }
        Ok(false)
    }

    /// env_file_path の内容が、name で指定したアーカイブと同じかどうかをチェックする
    pub async fn check_is_same_by_name(
        &self,
        name: &str,
        env_file_path: &Path,
    ) -> anyhow::Result<bool> {
        let checksum = crate::digest::file_checksum(env_file_path).await?;
        let conn = Connection::open(&self.database_path)?;
        let mut stmt = conn.prepare(
            "SELECT checksum FROM archives WHERE name = ?1 ORDER BY created_at DESC LIMIT 1",
        )?;
        let rows = stmt.query_map([name], |row| row.get::<_, String>(0))?;

        if let Some(row) = rows.into_iter().next() {
            let row = row?;
            return Ok(row == checksum);
        }
        Ok(false)
    }

    /// env_file_path の内容を、パスと時刻と共にアーカイブに登録する
    pub async fn push(
        &self,
        env_file_path: &Path,
        now: DateTime<Utc>,
        name: &str,
    ) -> anyhow::Result<()> {
        let body = tokio::fs::read_to_string(env_file_path).await?;
        let checksum = crate::digest::file_checksum(env_file_path).await?;

        let conn = Connection::open(&self.database_path)?;
        conn.execute(
            r#"
            INSERT INTO archives (name, path, created_at, body, checksum)
            VALUES (?1, ?2, ?3, ?4, ?5)
        "#,
            params![
                name,
                env_file_path.to_string_lossy(),
                now.to_rfc3339(),
                body,
                checksum,
            ],
        )?;

        Ok(())
    }

    pub async fn list_all(&self) -> anyhow::Result<Vec<ArchiveEntry>> {
        let conn = Connection::open(&self.database_path)?;
        let mut stmt = conn.prepare("SELECT name, path, created_at, checksum FROM archives")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;

        let mut archives = Vec::new();
        for row in rows {
            let (name, path, created_at, checksum) = row?;
            archives.push(ArchiveEntry {
                name,
                path,
                created_at: DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc),
                checksum,
            });
        }

        Ok(archives)
    }

    #[allow(dead_code)]
    pub async fn list_in_path(&self, path: &Path) -> anyhow::Result<Vec<ArchiveEntry>> {
        let conn = Connection::open(&self.database_path)?;
        let mut stmt = conn
            .prepare("SELECT name, path, created_at, checksum FROM archives WHERE path LIKE ?1")?;
        let rows = stmt.query_map([format!("{}%", path.to_string_lossy())], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;

        let mut archives = Vec::new();
        for row in rows {
            let (name, path, created_at, checksum) = row?;
            archives.push(ArchiveEntry {
                name,
                path,
                created_at: DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc),
                checksum,
            });
        }

        Ok(archives)
    }

    #[allow(dead_code)]
    pub async fn find_by_path(&self, path: &Path) -> anyhow::Result<Vec<ArchiveEntry>> {
        let conn = Connection::open(&self.database_path)?;
        let mut stmt = conn.prepare(
            "SELECT name, path, created_at, body, checksum FROM archives WHERE path = ?1 ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([path.to_string_lossy()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;

        let mut archives = Vec::new();
        for row in rows {
            let row = row?;
            archives.push(ArchiveEntry {
                name: row.0,
                path: row.1,
                created_at: DateTime::parse_from_rfc3339(&row.2)?.with_timezone(&Utc),
                checksum: row.3,
            });
        }
        Ok(archives)
    }

    /// name に一致するアーカイブを取得する
    pub async fn get(&self, name: &str) -> anyhow::Result<Option<(ArchiveEntry, String)>> {
        let conn = Connection::open(&self.database_path)?;
        let mut stmt = conn.prepare(
            "SELECT name, path, created_at, body, checksum FROM archives WHERE name = ?1 ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([name], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;

        let mut archives = Vec::new();
        for row in rows {
            let row = row?;
            archives.push((
                ArchiveEntry {
                    name: row.0,
                    path: row.1,
                    created_at: DateTime::parse_from_rfc3339(&row.2)?.with_timezone(&Utc),
                    checksum: row.4,
                },
                row.3,
            ));
        }
        Ok(archives.into_iter().next())
    }

    /// ファイルパスに keyword が部分一致するアーカイブを取得する
    pub async fn search(&self, keyword: &str) -> anyhow::Result<Vec<ArchiveEntry>> {
        let conn = Connection::open(&self.database_path)?;
        let mut stmt = conn.prepare(
            "SELECT name, path, created_at, checksum FROM archives WHERE path LIKE ?1 ORDER BY path, created_at DESC",
        )?;
        let rows = stmt.query_map([format!("%{}%", keyword)], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;

        let mut archives = Vec::new();
        for row in rows {
            let row = row?;
            archives.push(ArchiveEntry {
                name: row.0,
                path: row.1,
                created_at: DateTime::parse_from_rfc3339(&row.2)?.with_timezone(&Utc),
                checksum: row.3,
            });
        }
        Ok(archives)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveEntry {
    pub name: String,
    pub path: String,
    pub created_at: DateTime<Utc>,
    pub checksum: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    async fn create_dot_env_file(files: &[(PathBuf, &str)]) {
        for file in files {
            let (path, content) = file;
            let parent = path.parent().unwrap();
            if !parent.exists() {
                fs::create_dir_all(parent).unwrap();
                {}
            }
            tokio::fs::write(path, content).await.unwrap();
        }
    }

    #[tokio::test]
    async fn pushするとdbに保存される() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let database_path = tmp_dir.path().join("test.db");
        let archive = Archive::new(database_path.clone());
        archive.initialize().await.unwrap();
        let env_file_path = tmp_dir.path().join(".env");

        create_dot_env_file(&[(env_file_path.clone(), "FOO=BAR")]).await;

        let now = Utc::now();
        archive
            .push(&env_file_path, now, "test-name")
            .await
            .unwrap();

        let conn = Connection::open(&database_path).unwrap();
        let mut stmt = conn.prepare("SELECT * FROM archives").unwrap();
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .unwrap();

        let row = rows.into_iter().next().unwrap().unwrap();
        assert_eq!(row.0, "test-name");
        assert_eq!(row.1, env_file_path.to_string_lossy());
        assert_eq!(row.2, now.to_rfc3339());
        assert_eq!(row.3, "FOO=BAR");
    }

    #[tokio::test]
    async fn list_allするとdbに保存されたすべてのアーカイブの一覧が取得できる() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let database_path = tmp_dir.path().join("test.db");
        let archive = Archive::new(database_path.clone());
        archive.initialize().await.unwrap();

        let env_files = [
            (tmp_dir.path().join(".env"), "FOO=FIRST"),
            (tmp_dir.path().join("test_a").join(".env"), "FOO=SECOND"),
            (
                tmp_dir.path().join("test_b").join("internal").join(".env"),
                "FOO=THIRD",
            ),
        ];
        create_dot_env_file(&env_files).await;

        let now = Utc::now();
        for (n, (env_file_path, _)) in env_files.iter().enumerate() {
            archive
                .push(env_file_path, now, n.to_string().as_str())
                .await
                .unwrap();
        }

        let archives = archive.list_all().await.unwrap();
        assert_eq!(archives.len(), 3);
        for (
            i,
            ArchiveEntry {
                name,
                path,
                created_at,
                checksum,
            },
        ) in archives.iter().enumerate()
        {
            assert_eq!(name, &i.to_string());
            assert_eq!(path, &env_files[i].0.to_string_lossy());
            assert_eq!(created_at, &now);
            assert_eq!(
                checksum,
                &crate::digest::file_checksum(&env_files[i].0).await.unwrap()
            );
        }
    }

    #[tokio::test]
    async fn list_in_pathするとpath配下のアーカイブの一覧が取得できる() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let database_path = tmp_dir.path().join("test.db");
        let archive = Archive::new(database_path.clone());
        archive.initialize().await.unwrap();

        let env_files = [
            (tmp_dir.path().join(".env"), "FOO=FIRST"),
            (tmp_dir.path().join("test_a").join(".env"), "FOO=SECOND"),
            (
                tmp_dir.path().join("test_b").join("internal").join(".env"),
                "FOO=THIRD",
            ),
        ];
        create_dot_env_file(&env_files).await;

        let now = Utc::now();
        for (n, (env_file_path, _)) in env_files.iter().enumerate() {
            archive
                .push(env_file_path, now, n.to_string().as_str())
                .await
                .unwrap();
        }

        let archives = archive.list_in_path(tmp_dir.path()).await.unwrap();
        assert_eq!(archives.len(), 3);
        for (
            i,
            ArchiveEntry {
                name,
                path,
                created_at,
                checksum,
            },
        ) in archives.iter().enumerate()
        {
            assert_eq!(name, &i.to_string());
            assert_eq!(path, &env_files[i].0.to_string_lossy());
            assert_eq!(created_at, &now);
            assert_eq!(
                checksum,
                &crate::digest::file_checksum(&env_files[i].0).await.unwrap()
            );
        }

        let archives = archive
            .list_in_path(&tmp_dir.path().join("test_a"))
            .await
            .unwrap();
        assert_eq!(archives.len(), 1);
        assert_eq!(archives[0].name, "1");
        assert_eq!(archives[0].path, env_files[1].0.to_string_lossy());
        assert_eq!(archives[0].created_at, now);
    }

    #[tokio::test]
    async fn find_by_pathするとpathに一致するアーカイブの一覧が取得できる() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let database_path = tmp_dir.path().join("test.db");
        let archive = Archive::new(database_path.clone());
        archive.initialize().await.unwrap();

        let env_files = [
            (tmp_dir.path().join(".env"), "FOO=FIRST"),
            (tmp_dir.path().join("test_a").join(".env"), "FOO=SECOND"),
            (
                tmp_dir.path().join("test_b").join("internal").join(".env"),
                "FOO=THIRD",
            ),
        ];
        create_dot_env_file(&env_files).await;

        let now = Utc::now();
        for (n, (env_file_path, _)) in env_files.iter().enumerate() {
            archive
                .push(env_file_path, now, n.to_string().as_str())
                .await
                .unwrap();
        }

        let archives = archive
            .find_by_path(&tmp_dir.path().join(".env"))
            .await
            .unwrap();
        assert_eq!(archives.len(), 1);
        for (i, archive) in archives.iter().enumerate() {
            assert_eq!(archive.name, i.to_string());
            assert_eq!(archive.path, env_files[i].0.to_string_lossy());
            assert_eq!(archive.created_at, now);
        }

        let archives = archive
            .find_by_path(&tmp_dir.path().join("test_a").join(".env"))
            .await
            .unwrap();
        assert_eq!(archives.len(), 1);
        assert_eq!(archives[0].name, "1");
        assert_eq!(archives[0].path, env_files[1].0.to_string_lossy());
        assert_eq!(archives[0].created_at, now);
    }

    #[tokio::test]
    async fn getするとnameに一致するアーカイブが取得できる() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let database_path = tmp_dir.path().join("test.db");
        let archive = Archive::new(database_path.clone());
        archive.initialize().await.unwrap();

        let env_files = [
            (tmp_dir.path().join(".env"), "FOO=FIRST"),
            (tmp_dir.path().join("test_a").join(".env"), "FOO=SECOND"),
            (
                tmp_dir.path().join("test_b").join("internal").join(".env"),
                "FOO=THIRD",
            ),
        ];
        create_dot_env_file(&env_files).await;

        let now = Utc::now();
        for (n, (env_file_path, _)) in env_files.iter().enumerate() {
            archive
                .push(env_file_path, now, n.to_string().as_str())
                .await
                .unwrap();
        }

        let (entry, body) = archive.get("1").await.unwrap().unwrap();
        assert_eq!(entry.name, "1");
        assert_eq!(entry.path, env_files[1].0.to_string_lossy());
        assert_eq!(entry.created_at, now);
        assert_eq!(body, env_files[1].1);
    }

    #[tokio::test]
    async fn searchするとkeywordに一致するアーカイブが取得できる() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let database_path = tmp_dir.path().join("test.db");
        let archive = Archive::new(database_path.clone());
        archive.initialize().await.unwrap();

        let env_files = [
            (tmp_dir.path().join(".env"), "FOO=FIRST"),
            (tmp_dir.path().join("test_a").join(".env"), "FOO=SECOND"),
            (
                tmp_dir.path().join("test_b").join("internal").join(".env"),
                "FOO=THIRD",
            ),
        ];
        create_dot_env_file(&env_files).await;

        let now = Utc::now();
        for (n, (env_file_path, _)) in env_files.iter().enumerate() {
            archive
                .push(env_file_path, now, n.to_string().as_str())
                .await
                .unwrap();
        }

        let archives = archive.search("test_").await.unwrap();
        assert_eq!(archives.len(), 2);

        let archives = archive.search("test_a").await.unwrap();
        assert_eq!(archives.len(), 1);
        assert_eq!(archives[0].name, "1");
        assert_eq!(archives[0].path, env_files[1].0.to_string_lossy());
        assert_eq!(archives[0].created_at, now);
    }
}
