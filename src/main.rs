// コマンドの説明
// .env ファイルをアーカイブしたり、アーカイブから取り出して .env ファイルを復元したりするコマンドです。
// アーカイブされた .env ファイルはもとのファイルパスとアーカイブした日時によりタグ付けされます。
// タグ付けされた .env ファイルは一意に識別できるため、同じファイルを複数回アーカイブしても問題ありません。

mod archive;
mod digest;
mod helper;

use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

#[derive(Debug, Parser)]
#[clap(
    name = env!("CARGO_PKG_NAME"),
    version = env!("CARGO_PKG_VERSION"),
    author = env!("CARGO_PKG_AUTHORS"),
    about = env!("CARGO_PKG_DESCRIPTION"),
    arg_required_else_help = true,
)]
struct Args {
    #[clap(subcommand)]
    subcommand: SubCommands,
    /// アーカイブデータベースファイルのパス
    /// デフォルトは $HOME/.env_archive です
    #[clap(short, long, env = "ENV_ARCHIVE_DATABASE")]
    database: Option<String>,
}

#[derive(Debug, Subcommand)]
enum SubCommands {
    /// アーカイブを初期化する
    #[clap(arg_required_else_help = false)]
    Init {
        /// アーカイブを強制的に初期化する
        #[clap(long, default_value = "false")]
        clean: bool,
    },
    /// アーカイブに .env ファイルを登録する
    Push {
        /// アーカイブに登録する .env ファイルのパス
        #[clap(default_value = ".env")]
        file: String,
        /// 登録名
        #[clap(short, long)]
        name: Option<String>,
    },
    /// ディレクトリを再帰的に巡回して .env, .env.* ファイルを探し、アーカイブに登録する
    #[clap(arg_required_else_help = false)]
    Crawl {
        /// アーカイブに登録する .env ファイルを探すディレクトリ
        #[clap(short, long, default_value = ".")]
        dir: String,
        #[clap(long = "dry-run")]
        dry_run: bool,
    },
    /// アーカイブに登録されている .env ファイルをパス名の部分一致で検索する
    Search {
        /// アーカイブに登録されている .env ファイルパスの一部
        #[clap(required = true)]
        keyword: String,
    },
    /// カレントディレクトリ、または指定したパス配下に一致するアーカイブの一覧を表示する
    List {
        #[clap(short, long, default_value = ".")]
        dir: String,
    },
    /// アーカイブに登録されている .env ファイルの一覧を表示する
    ListAll,
    /// アーカイブに登録されている .env ファイルを表示する
    Show {
        /// アーカイブに登録されている .env ファイルの名前
        #[clap(required = true)]
        name: String,
    },
    /// アーカイブに登録されている .env ファイルを復元する
    Recover {
        /// アーカイブに登録されている .env ファイルの名前
        #[clap(required = true)]
        name: String,
    },
}

#[derive(Debug, Clone)]
struct Context {
    database: PathBuf,
    now: chrono::DateTime<chrono::Utc>,
    timezone: chrono_tz::Tz,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let database: PathBuf = args.database.map(PathBuf::from).unwrap_or_else(|| {
        dirs::home_dir()
            .expect("Failed to get home directory")
            .join(".env_archive")
    });

    let now = chrono::Utc::now();
    let context = Context {
        database,
        now,
        timezone: chrono_tz::Asia::Tokyo,
    };

    match args.subcommand {
        SubCommands::Crawl { dir, dry_run } => {
            crawl(&context, &std::fs::canonicalize(Path::new(&dir))?, dry_run).await;
        }
        SubCommands::Init { clean } => {
            init(&context, clean).await;
        }
        SubCommands::Push { file, name } => {
            push(&context, &std::fs::canonicalize(Path::new(&file))?, name).await;
        }
        SubCommands::List { dir } => {
            list(&context, &std::fs::canonicalize(Path::new(&dir))?).await;
        }
        SubCommands::ListAll => {
            list_all(&context).await;
        }
        SubCommands::Show { name } => {
            show(&context, &name).await;
        }
        SubCommands::Search { keyword } => {
            search(&context, keyword).await;
        }
        SubCommands::Recover { name } => {
            recover(&context, &name).await;
        }
    }

    Ok(())
}

async fn init(context: &Context, clean: bool) {
    if clean && context.database.exists() {
        std::fs::remove_file(&context.database).expect("Failed to remove archive");
    }
    let archive = archive::Archive::new(context.database.to_path_buf());
    archive
        .initialize()
        .await
        .expect("Failed to initialize archive");
}

async fn push(context: &Context, env_file_path: &Path, name: Option<String>) {
    let archive = archive::Archive::new(context.database.to_path_buf());
    archive
        .push(
            env_file_path,
            context.now,
            name.unwrap_or_else(|| {
                let ulid = ulid::Ulid::new();
                ulid.to_string()
            })
            .as_str(),
        )
        .await
        .expect("Failed to push archive");
}

async fn list_all(context: &Context) {
    // think 現状はすべてのタイムスタンプを出力しているが、最新のアーカイブのみを表示するコマンドとして
    // 過去のアーカイブを列挙するコマンドを別に切り出したほうが使いやすくなる
    let archive = archive::Archive::new(context.database.to_path_buf());
    let archives = archive.list_all().await.expect("Failed to list archive");
    for archive in archives {
        println!(
            "{} {:?} {}",
            archive.name,
            archive.path,
            archive.created_at.with_timezone(&context.timezone)
        );
    }
}

async fn list(context: &Context, path: &Path) {
    // think 現状はすべてのタイムスタンプを出力しているが、最新のアーカイブのみを表示するコマンドとして
    // 過去のアーカイブを列挙するコマンドを別に切り出したほうが使いやすくなる
    let archive = archive::Archive::new(context.database.to_path_buf());
    let archives = archive
        .list_in_path(path)
        .await
        .expect("Failed to list archive");
    for archive in archives {
        println!(
            "{} {:?} {}",
            archive.name,
            archive.path,
            archive.created_at.with_timezone(&context.timezone)
        );
    }
}

async fn show(context: &Context, name: &str) {
    let archive = archive::Archive::new(context.database.to_path_buf());
    let (_, body) = archive
        .get(name)
        .await
        .expect("Failed to show archive")
        .expect("Archive not found");
    println!("{}", body);
}

async fn recover(context: &Context, name: &str) {
    let archive = archive::Archive::new(context.database.to_path_buf());
    let (entry, body) = archive
        .get(name)
        .await
        .expect("Failed to show archive")
        .expect("Archive not found");
    let target_filename = Path::new(&entry.path)
        .file_name()
        .expect("Failed to get file name")
        .to_string_lossy()
        .to_string();
    let target_path = Path::new(&target_filename);
    println!(
        "archive_path: {}\ntarget_path: {:?}",
        entry.path, target_path
    );

    if target_path.exists() {
        if archive
            .check_is_same_by_name(name, target_path)
            .await
            .expect("Failed to check body")
        {
            println!("[SKIP] same checksum. {}", target_path.display());
            return;
        }
        let ulid = ulid::Ulid::new();
        let backup_name = format!("backup.{}", ulid.to_string());
        archive
            .push(target_path, context.now, &backup_name)
            .await
            .expect("Failed to push archive for backup");
        println!(
            "[BACKUP] {} with name {}",
            target_path.display(),
            backup_name
        );
    }

    std::fs::write(target_path, body).expect("Failed to write file");
    println!("[RECOVERED] {} from {}", target_path.display(), name);
}

async fn crawl(context: &Context, dir: &Path, dry_run: bool) {
    let files = helper::search_env_files(dir).expect("Failed to search env files");

    let archive = archive::Archive::new(context.database.to_path_buf());
    for file in files {
        let name = ulid::Ulid::new().to_string();
        if archive
            .check_is_same_as_latest(&file)
            .await
            .expect("Failed to check body")
        {
            println!("[SKIP] {}", file.display());
            continue;
        }
        if dry_run {
            println!("[PUSH DRY RUN] {}", file.display());
            continue;
        }
        archive
            .push(&file, context.now, &name)
            .await
            .expect("Failed to push archive");
        println!("[PUSHED] {}", file.display());
    }
}

async fn search(context: &Context, keyword: String) {
    let archive = archive::Archive::new(context.database.to_path_buf());
    let archives = archive
        .search(&keyword)
        .await
        .expect("Failed to search archive");
    for archive in archives {
        println!(
            "{} {:?} {}",
            archive.name,
            archive.path,
            archive.created_at.with_timezone(&context.timezone)
        );
    }
}
