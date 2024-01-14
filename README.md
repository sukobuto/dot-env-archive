# dot-env-archive

## summary

- .env ファイルをアーカイブしたり、アーカイブから取り出して .env ファイルを復元したりするコマンドです。
- アーカイブされた .env ファイルはもとのファイルパスとアーカイブした日時によりタグ付けされます。
- タグ付けされた .env ファイルは一意に識別できるため、同じファイルを複数回アーカイブしても問題ありません。
- ディレクトリを再起に検索し、すべての `.env` ファイルをアーカイブに登録する機能があります。開発環境の移行等に便利です。

## detail

- `.env` の他に `.env.local` などのファイルも拾います。
- アーカイブはデフォルトで `$HOME/.env-archive` ファイルに記録されます。SQLite のデータベースファイルなので、テキストエディタ等により直接編集することはできません。

# setup

```
cargo install dot-env-archive
```

# usage

```
Usage: dot-env-archive [OPTIONS] <COMMAND>

Commands:
  init      アーカイブを初期化する
  push      アーカイブに .env ファイルを登録する
  crawl     ディレクトリを再帰的に巡回して .env, .env.* ファイルを探し、アーカイブに登録する
  search    アーカイブに登録されている .env ファイルをパス名の部分一致で検索する
  list      カレントディレクトリ、または指定したパス配下に一致するアーカイブの一覧を表示する
  list-all  アーカイブに登録されている .env ファイルの一覧を表示する
  show      アーカイブに登録されている .env ファイルを表示する
  recover   アーカイブに登録されている .env ファイルを復元する
  help      Print this message or the help of the given subcommand(s)

Options:
  -d, --database <DATABASE>  アーカイブデータベースファイルのパス デフォルトは $HOME/.env_archive です [env: ENV_ARCHIVE_DATABASE=]
  -h, --help                 Print help
  -V, --version              Print version
```