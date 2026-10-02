# MaXImDock v2

Windows 向けの軽量な常駐 Dock ランチャー。Rust / egui / eframe と Windows Shell を使用する。

Phase 1 の実装済み PoC。主モニターの指定辺から復帰する 1 ウィンドウと、Explorer / Home / GitHub の固定 3 項目を備える。Phase 1 の実機手動受け入れは一部未確認。Phase 2/3 の機能別状況と検証結果は [acceptance](docs/acceptance.md) と [notes](docs/notes.md) を参照。

前提: Windows 10 以降、x86_64、Rust stable MSVC、Visual Studio Build Tools の C++ ツールチェーンと Windows SDK。検証時の Rust は 1.95.0。依存解決は Cargo.lock をコミットして固定する。

```powershell
cargo build --locked
cargo run --locked
cargo run --locked -- --edge top
cargo run --locked -- --edge left
cargo run --locked -- --edge right
```

画面端の 2 物理 px にカーソルを置くと Dock を表示する。範囲外に出てから 500 ms で非表示、再表示直後の 200 ms は保持する。Dock にフォーカスがある時の Esc、または背景の右クリック → Hide で非表示。背景の右クリック → Quit で終了する。隠れた後も選択辺から復帰できる。非アクティブで表示するため Esc はグローバルキーではない。

Windows Shell 経由で `.exe`、`.lnk`、フォルダ、関連付け済みファイル、http/https を開くアダプターを備える。UI からは固定 3 項目のみ起動可能。欠損パスと基本的な不正 URL は Shell 呼び出し前に拒否する。

設定ファイルは Phase 1 では読み書きしない。Phase 2 の予定保存先は `%APPDATA%\MaXImDock\config.toml`。辺の CLI 指定も再起動後に保存しない。

```powershell
cargo fmt --check
cargo clippy -- -D warnings
cargo test
$env:RUST_LOG = 'maxim_dock_rs=debug'
cargo run --locked
# カーソルを動かさず、実ウィンドウの30回 hide/show を約18秒で検証し終了
cargo run --locked -- --smoke-test
```

smoke は本物の HWND 非表示・再表示、非表示中 GetCursorPos、再表示後の描画と配置を検査する。ホットゾーン進入は疑似入力であり、実際のマウス操作、Shell 起動、フォーカス、150% DPI の受け入れを代用しない。

既知の制約: 主モニターのみ、文字ボタンのみ、UI は英語、設定・編集・通知領域・DnD・Shell アイコン・拡大表示は未実装。起動エラーは小さな Dock 内に表示する簡易 UI。DPI の手動確認、タスクバー・フルスクリーン・スリープ復帰の相互作用は未検証。
