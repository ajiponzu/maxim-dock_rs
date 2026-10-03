# MaXImDock v2

Windows 向けの軽量な常駐 Dock ランチャー。Rust / egui / eframe と Windows Shell を使用する。

Phase 1〜3 の機能を実装済み（混在 DPI・実操作の手動受け入れは未完了）。新規設定の表示位置は **Top**。設定がある場合は保存した辺・タイミング・表示設定・順序付き項目を復元する。自動検証と手動未検証の区別は [acceptance](docs/acceptance.md)、設計判断と依存理由は [notes](docs/notes.md) を参照。

前提: Windows 10 以降、x86_64、Rust stable MSVC、Visual Studio Build Tools の C++ ツールチェーンと Windows SDK。検証時 Rust 1.95.0。Cargo.lock で依存解決を固定する。

```powershell
cargo build --locked
cargo run --locked
# 一時的な辺指定。設定画面で Apply するまではファイルに保存しない
cargo run --locked -- --edge bottom
```

配布用 exe とインストーラーは `installer\build.ps1` で生成できます（Rust stable MSVC と Inno Setup 6.7.3 が必要です）。出力は `dist\MaXIMDock.exe` と `dist\MaXImDock-v2-Setup-x64.exe` です。インストーラーはユーザー単位で導入し、スタートメニューに登録します。デスクトップアイコンは任意で選択できます。アンインストール時はアプリ本体とショートカットを削除し、ユーザー設定は保持します。同じ AppId で更新し、インストール先の旧 `MaXImDock-v2-x86_64.exe` を除去します。Inno Setup 6 のコンパイラーは非商用利用向けです。商用配布では Inno Setup のライセンス条件を確認してください。

release はコンソールを表示しない Windows GUI executable です。外部プロセスからの停止は PID、または `Stop-Process -Name MaXIMDock -Force` を使用できます。親アプリの登録パス／停止に使う名前も新しいファイル名に合わせてください。呼び出し元のソースは変更しません。

上端の 2 物理 px にカーソルを置くと Dock を表示する。範囲外に出てから既定 500 ms で非表示、再表示直後の 200 ms は保持する。Dock にフォーカスがある時の Esc、または右クリック → Hide で非表示。Esc はグローバルキーではない。表示中にクリックでき、復帰時に不要なフォーカス取得を避ける。

Dock の歯車「設定」、または右クリック → 設定で別ウィンドウの設定画面を開く。設定を閉じてもアプリは継続する。設定を開いている間は自動 hide を保留する。明示的に隠した Dock と設定ウィンドウは共存できる。

Dock は角丸パネルと大きめのアイコン、16 pt のラベルを使用する。設定画面は「Dock／アイテム／作業環境／詳細」の4ページで、本文・ボタンは17 pt、補足は15 pt。配色はシステム／ライト／ダークに対応し、下部の「適用して保存」「変更を破棄」はスクロールしても表示される。Windows の DPI に従って拡大し、独自のズームは変更しない。

歯車の隣（縦 Dock では上側）の四分割アイコンで、アプリ一覧と作業環境一覧を切り替えます。両ボタンは36px。設定の「作業環境」で名前付きの起動レシピを作成し、各アプリの実行ファイル・引数・作業ディレクトリ・モニター・配置を保存できます。ファイル／URL／フォルダーは個別の引数として指定します。配置は最大化・上下左右半分・四隅・カスタム割合に対応。保存後に作業環境カードをクリックすると順に起動・配置します。[設定と制約](docs/workspaces.md)を参照してください。

作業環境に「Windows Terminal モード」を追加できます。PowerShell／コマンドプロンプト／WSL (bash) と起動コマンドを選択し、Python等の対話アプリを開いた状態にできます。WSLで作業フォルダーと `codex` を設定すれば、そのフォルダーで起動します。Terminalの新規ウィンドウを自動識別して、通常と同じモニター・配置を使用します。[Terminal の設定例と制約](docs/workspaces.md#windows-terminal-モード)を参照してください。

設定では次の操作ができる。

作業環境の各エントリーで「WSL モード」を選択できます。既定は普段の `wsl code` と同じ「WSL 標準シェル」で、引数は個別に安全に引用します。必要ならbashログインシェル／直接実行も選べます。「起動した Windows ウィンドウを配置する」を選び、Windows側の `Code.exe` 等を指定すると同じモニター・配置を利用できます。成功終了を待って次へ進み、失敗時は診断出力を表示して後続を止めます。未保存のレシピ変更がある場合は古い設定で起動せず通知します。コンソール表示・Linux WSLgウィンドウの配置は行いません。[WSL モードの設定](docs/workspaces.md#wsl-モード)を参照してください。

カラーテーマは標準／Ocean／Forest／Rose／カスタムから選べます。カスタムは色ボタンや `#RRGGBB`、TOMLで編集可能です。CSS は使用しません。[テーマの設定方法](docs/color-themes.md)を参照してください。

- 表示名の編集、Up / Down の順序変更、Delete
- Choose files（複数ファイル）/ Choose folder のネイティブダイアログ
- URL / path の直接入力追加。同じ target は重複登録しない
- Top / Bottom / Left / Right、アイコンサイズ、間隔、auto hide、hide delay、hot-zone 幅、polling、reveal hold、最前面、不透明度、基本 UI theme
- Open config folder、Show Dock、Quit app

変更は **適用して保存** で検証・保存してから反映する。可視 Dock の辺・サイズは即再配置、隠れた Dock は次回表示で適用する。保存はフレームごとには行わない。多数の項目は Dock の主軸方向にスクロールする。

設定保存先は `%APPDATA%\MaXImDock\config.toml`。初回に Explorer / 現在ユーザーの Home / GitHub を登録する。`--config <path>` で別の設定ファイルを指定できる。

不正・読込不可・未対応バージョンの設定は保持し、Top の安全な既定値で起動して設定画面にエラーを出す。不正ファイルへの保存は無効になる。画面の明示的なバックアップ操作で `config.invalid-<UUID>.toml` に元のバイトを保管した後、Apply and save で設定を置換できる。外部編集との競合時は保存を拒否するので、再起動して読み直す。

Windows Shell 経由で `.exe`、`.lnk`、フォルダ、関連付け済みファイル、http/https を開く。欠損パスは起動時にエラーを表示する。保存済みの欠損パスは取り外したドライブ等を考慮して保持する。URL は http/https と有効なホストのみ、認証情報付き URL は拒否する。

アイコンを取得できない項目には、コード描画の安定した fallback を表示する。Windows のローカル日本語フォントを利用し、フォントがない場合は同梱フォントで継続する。日本語フォントをリポジトリに配布しない。

Shell アイコンは最大256×256pxを要求し、Dock のサイズへ縮小表示します。既定ブラウザーも同じ方式です。高解像度取得に失敗した場合は従来の Shell アイコンへ戻ります。元アプリが低解像度の画像しか持たない場合、細部の鮮明さは保証できません。

通知領域のメニューから Show Dock / Settings / Quit を操作できる。非表示時はカーソルがあるモニターの選択辺で復帰し、表示中はそのモニターに固定する。対象モニターへ移動後に DPI を取得して配置する。

表示中の Dock のアプリカード（`.exe`／アプリを指す `.lnk`）へファイルをドロップすると、そのアプリで開きます。複数ファイル、日本語・空白を含むパスに対応。受け取り可能なカードは強調表示します。ファイル引数に対応しないアプリでは開けない場合があります。登録は設定から行い、Dockへの外部ドロップでは登録・保存・コピー・移動しません。URL・フォルダー・空白へのドロップは案内のみです。

Dock のカードはドラッグして並べ替えできます。横Dockは左右、縦Dockは上下へ移動し、挿入線の位置へドロップすると保存します。ドラッグ中は自動非表示を保留し、Esc または Dock 外へのドロップで取消します。未保存の設定がある場合は先に適用／破棄してください。設定の「上へ／下へ」も引き続き利用できます。

Windows Shell アイコンを background worker で抽出しキャッシュする。設定のアイコン画像に PNG/ICO/JPEG のパスを指定でき、失敗時は Shell / fallback に戻る。URL の自動アイコンは http／https に関連付けられた既定ブラウザーのアイコンを使用し、取得できなければ地球アイコンへ戻る。明示した画像・組み込みアイコンは引き続き優先する。ブラウザー変更や画像変更後は「詳細 → アイコンを再読み込み」、または再起動で更新する。欠損パス等は安定した fallback を使う。

```powershell
cargo fmt --check
cargo clippy -- -D warnings
cargo test
$env:RUST_LOG = 'maxim_dock_rs=debug'
cargo run --locked
# 約20秒で検証し正常終了。実設定には触れない
cargo run --locked -- --smoke-test
# 任意: Dock と設定4ページの描画確認用 PNG を保存
$env:MAXIMDOCK_SMOKE_CAPTURE_DIR = 'target/ui-review'
cargo run --locked --target-dir target/refactor-smoke -- --smoke-test
Remove-Item Env:MAXIMDOCK_SMOKE_CAPTURE_DIR
# 専用 GUI fixture の起動・配置・中止（実ユーザーアプリは起動しない）
cargo test --locked native_workspace_launch_placement_timeout_cancel_and_existing_window_safety -- --ignored --nocapture
# Terminal専用テスト: PowerShell/cmd/Ubuntu-24.04の無害なスクリプトと新規ウィンドウ配置
cargo test --locked native_terminal_scripts_run_in_requested_directories_and_preserve_existing_windows -- --ignored --nocapture
# release の PID／プロセス名停止（既存 MaXIMDock があれば検証を拒否）
./tests/parent-control.ps1
```

smoke は一時設定だけを使う。Top で起動し、四辺への Apply/save/reload、可視 Dock の即再配置、実 HWND の30回 hide/show、hidden の GetCursorPos、描画再開、hidden root と設定 child の共存、設定閉鎖後の継続を検査する。ホットゾーン進入と UI コマンドは疑似入力。クリック操作・ネイティブダイアログ・実カーソルの端進入・150% DPI の手動受け入れは未検証。

Phase 3 の未検証: 実トレイクリック、Explorer DnD、実マウスでのカード並べ替え、混在 DPI の複数モニター、負座標モニター、スリープ/ロック/フルスクリーンとの相互作用。smoke は実 tray 作成と疑似メニュー／ドロップ command も検査するが実操作の代用ではない。未実装: モニター切断への完全自動追従、モノクロ/MAX_PATH超の Shell icon の特殊対応（fallback使用）。ログイン起動は設計調査のみで OS 登録は変更しない。高度な hover 拡大・スタートアップ登録は Phase 4。
