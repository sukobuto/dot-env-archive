// コマンドの説明
// .env ファイルをアーカイブしたり、アーカイブから取り出して .env ファイルを復元したりするコマンドです。
// アーカイブされた .env ファイルはもとのファイルパスとアーカイブした日時によりタグ付けされます。
// タグ付けされた .env ファイルは一意に識別できるため、同じファイルを複数回アーカイブしても問題ありません。

mod archive;

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
    #[clap(short, long, env = "ENV_ARCHIVE_DATABASE")]
    database: Option<String>,
}

#[derive(Debug, Subcommand)]
enum SubCommands {
    /// アーカイブに登録する .env ファイルを探す
    #[clap(arg_required_else_help = false)]
    Crawl {
        /// アーカイブに登録する .env ファイルを探すディレクトリ
        #[clap(short, long, default_value = ".")]
        dir: String,
        #[clap(long = "dry-run")]
        dry_run: bool,
    },
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
    /// アーカイブに登録されている .env ファイルの一覧を表示する
    ListAll,
    /// アーカイブに登録されている .env ファイルを表示する
    Show {
        /// アーカイブに登録されている .env ファイルの名前
        #[clap(required = true)]
        name: String,
    },
    /// アーカイブに登録されている .env ファイルをパス名の部分一致で検索する
    Search {
        /// アーカイブに登録されている .env ファイルパスの一部
        #[clap(required = true)]
        keyword: String,
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

    let database: PathBuf = args.database.map(|d| PathBuf::from(d)).unwrap_or_else(|| {
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
        SubCommands::ListAll => {
            list_all(&context).await;
        }
        SubCommands::Show { name } => {
            show(&context, name).await;
        }
        SubCommands::Search { keyword } => {
            search(&context, keyword).await;
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

async fn show(context: &Context, name: String) {
    let archive = archive::Archive::new(context.database.to_path_buf());
    let (_, body) = archive
        .get(&name)
        .await
        .expect("Failed to show archive")
        .expect("Archive not found");
    println!("{}", body);
}

async fn crawl(context: &Context, dir: &Path, dry_run: bool) {
    // todo dir を再帰的に巡回して .env ファイルを探し、アーカイブに登録する
    let files = globmatch::Builder::new("**/.env")
        .build(dir)
        .expect("Failed to build globmatch")
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    if dry_run {
        for file in files {
            println!("{}", file.display());
        }
        return;
    }

    let archive = archive::Archive::new(context.database.to_path_buf());
    for file in files {
        let name = ulid::Ulid::new().to_string();
        archive
            .push(&file, context.now, &name)
            .await
            .expect("Failed to push archive");
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
