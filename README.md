# MaXImDock v2

Windows 向けの軽量な常駐 Dock ランチャー。Rust / egui / eframe と Windows Shell を使用する。

Phase 1 と Phase 2 の実装済み。新規設定の表示位置は **Top**。設定がある場合は保存した辺・タイミング・表示設定・順序付き項目を復元する。自動検証と手動未検証の区別は [acceptance](docs/acceptance.md)、設計判断と依存理由は [notes](docs/notes.md) を参照。

前提: Windows 10 以降、x86_64、Rust stable MSVC、Visual Studio Build Tools の C++ ツールチェーンと Windows SDK。検証時 Rust 1.95.0。Cargo.lock で依存解決を固定する。

```powershell
cargo build --locked
cargo run --locked
# 一時的な辺指定。設定画面で Apply するまではファイルに保存しない
cargo run --locked -- --edge bottom
```

上端の 2 物理 px にカーソルを置くと Dock を表示する。範囲外に出てから既定 500 ms で非表示、再表示直後の 200 ms は保持する。Dock にフォーカスがある時の Esc、または右クリック → Hide で非表示。Esc はグローバルキーではない。表示中にクリックでき、復帰時に不要なフォーカス取得を避ける。

Dock の `...`、または右クリック → Settings で別ウィンドウの設定画面を開く。設定を閉じてもアプリは継続する。設定を開いている間は自動 hide を保留する。明示的に隠した Dock と設定ウィンドウは共存できる。

設定では次の操作ができる。

- 表示名の編集、Up / Down の順序変更、Delete
- Choose files（複数ファイル）/ Choose folder のネイティブダイアログ
- URL / path の直接入力追加。同じ target は重複登録しない
- Top / Bottom / Left / Right、アイコンサイズ、間隔、auto hide、hide delay、hot-zone 幅、polling、reveal hold、最前面、不透明度、基本 UI theme
- Open config folder、Show Dock、Quit app

変更は **Apply and save** で検証・保存してから反映する。可視 Dock の辺・サイズは即再配置、隠れた Dock は次回表示で適用する。保存はフレームごとには行わない。多数の項目は Dock の主軸方向にスクロールする。

設定保存先は `%APPDATA%\MaXImDock\config.toml`。初回に Explorer / 現在ユーザーの Home / GitHub を登録する。`--config <path>` で別の設定ファイルを指定できる。

不正・読込不可・未対応バージョンの設定は保持し、Top の安全な既定値で起動して設定画面にエラーを出す。不正ファイルへの保存は無効になる。画面の明示的なバックアップ操作で `config.invalid-<UUID>.toml` に元のバイトを保管した後、Apply and save で設定を置換できる。外部編集との競合時は保存を拒否するので、再起動して読み直す。

Windows Shell 経由で `.exe`、`.lnk`、フォルダ、関連付け済みファイル、http/https を開く。欠損パスは起動時にエラーを表示する。保存済みの欠損パスは取り外したドライブ等を考慮して保持する。URL は http/https と有効なホストのみ、認証情報付き URL は拒否する。

全項目にコード描画の安定した fallback アイコンを表示する。Windows のローカル日本語フォントを利用し、フォントがない場合は同梱フォントで継続する。日本語フォントをリポジトリに配布しない。

```powershell
cargo fmt --check
cargo clippy -- -D warnings
cargo test
$env:RUST_LOG = 'maxim_dock_rs=debug'
cargo run --locked
# 約20秒で検証し正常終了。実設定には触れない
cargo run --locked -- --smoke-test
```

smoke は一時設定だけを使う。Top で起動し、四辺への Apply/save/reload、可視 Dock の即再配置、実 HWND の30回 hide/show、hidden の GetCursorPos、描画再開、hidden root と設定 child の共存、設定閉鎖後の継続を検査する。ホットゾーン進入と UI コマンドは疑似入力。クリック操作・ネイティブダイアログ・実カーソルの端進入・150% DPI の手動受け入れは未検証。

Phase 3 の未実装: トレイ、マルチモニター/mixed DPI、外部 DnD、Shell アイコン抽出/キャッシュ、カスタム画像アイコン、ログイン起動の設計調査。主モニターのみで動作する。スリープ/ロック/フルスクリーンとの相互作用は未検証。高度な hover 拡大は Phase 4。
