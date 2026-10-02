# 実装・検証記録

## UI 責務分離（2026-10-03）

肥大化した ui/mod.rs から責務を分離した。公開入口 `ui::DockApp`、CLI、設定スキーマ、Top 既定値、hide/reveal と保存の挙動は維持。依存追加なし。

| モジュール | 責務 |
| --- | --- |
| ui/mod.rs | モジュール宣言と DockApp の再公開だけ |
| ui/app.rs | App の所有状態、初期化、eframe logic/ui、表示状態とビューの接続 |
| ui/app/actions.rs | UI command の実行、保存・起動・トレイ／選択結果の接続 |
| ui/app/smoke.rs | native smoke の検証シナリオと計測。通常動作とは分離 |
| ui/poll_wake.rs | hidden-only repaint wake、atomic state、thread の停止／join |
| ui/item_import.rs | ドロップ項目の検証・順序付き draft 作成とそのテスト |
| platform_windows/file_dialog.rs | rfd worker、受信状態、多重起動防止、選択／取消／切断の結果 |

actions/smoke は App の子モジュールとして必要な接続処理をまとめる。App の field は private のままで、分割のために広い公開 API や getter/setter を増やさない。PollWake は set_hidden だけを公開し atomic を外へ出さない。FilePicker は egui、設定、HWND を保持せず、呼出側から渡された wake callback と owned PathBuf だけで橋渡しする。設定画面と Dock の描画は従来の settings/dock_view、Shell/GDI は Windows 層を維持する。

検証: fmt/clippy -D warnings/test 成功、23 tests（既存21 + picker の pending/複数起動拒否/順序/取消/切断2 tests）。native smoke は起動中のユーザー Dock が target/debug exe を使用していて通常出力先のビルドが Access denied になったため、既存プロセスを終了せず `--target-dir target/refactor-smoke` を使用。実 hide/show30回、hidden poll92回、描画641 frames、NATIVE/PHASE2/PHASE3_SMOKE_PASS、終了コード0。既存の手動未検証項目は変更しない。

## Phase 3 依存・方式選定（実装前）

- tray-icon: Windows の UI thread 上で生成・保持し、既存 winit の message loop を使用する。独自 Shell_NotifyIcon/subclass 実装より所有権管理を委譲する。メニュー callback は channel と repaint のみで UI state を変更しない。初期化失敗は非致命的。
- モニターは MonitorFromPoint / GetMonitorInfoW。表示中の選択は固定し、hidden のホットゾーン進入だけで更新する。別モニターへの非表示移動後に GetDpiForWindow を取得し、points → px を platform だけで変換する。
- 外部 DnD は egui RawInput.dropped_files を利用し、大きな DnD framework は追加しない。通常の検証・競合検出・保存経路を再利用する。未適用の設定 draft がある場合はドロップを拒否して編集を保護する。
- image 0.25（png/ico/jpeg のみ）: 指定画像のデコード。eframe の transitive image と同じ版を使用し、独自画像パーサーは避ける。サイズ/デコード割当制限を適用する。windows の System_Com / System_Threading は Shell 抽出 worker の COM 初期化と resource-count テスト用。
- Shell 抽出は SHGetFileInfoW → GetIconInfo → GetDIBits。所有 HICON / color・mask bitmap / memory DC を RAII で解放する。IShellItemImageFactory の thumbnail/COM interface 管理より最小の icon-only 経路を選ぶ。抽出は worker 上だけ、UI は画像をキャッシュする。


## Phase 2 依存選定（実装前）

- serde 1 + toml 1: 指定された version 付き設定の読み書き。独自パーサーよりスキーマと往復検証を優先。
- uuid 1（serde/v4）: 編集・並び替え後も安定した項目 ID。配列添字を ID にしない。
- rfd 0.17: 指定されたネイティブファイル/フォルダ追加ダイアログ。Win32 COM を自前実装しない。
- url 2: http/https の構文と重複キーを検証。文字列の前方一致だけでは不正ホストを見逃すため採用。
- tempfile 3: 実設定を触らず保存・破損復帰をテストする独立ディレクトリ。native smoke も一時設定で Apply/save と再読込を検証するため runtime で使用。
- windows の Storage_FileSystem feature: 同一ディレクトリの一時ファイルを MoveFileExW で置換。remove+rename の消失区間を避ける。

ユーザー指示により既定辺は Top。保存済みの明示的な辺は尊重する。設定画面は別 viewport、変更は Apply and save により検証・保存後に反映。破損設定の上書きは明示的なバックアップ操作まで禁止する。

## Phase 1 の設計と依存選定（2026-10-02）

単一 crate、`src/core` / `src/platform_windows` / `src/ui` の境界を維持する。
依存を追加する前に以下の選定を記録した。

| 依存 | 目的・代替案 |
| --- | --- |
| eframe 0.36.2 / 同梱 egui | 必須 GUI。標準 winit イベントループを利用。hidden 時にも `App::logic` が repaint に応答する版を採用。独自イベントループは不要 |
| windows 0.62.2 | 必須の型付き Win32 bindings。windows-sys の手動エラー処理より小さな platform adapter を優先 |
| thiserror 2 | platform エラーの原因を保持。文字列だけのエラーを避ける |
| tracing / tracing-subscriber | 指定のログと RUST_LOG 設定。独自 logger を避ける |
| raw-window-handle 0.6 | eframe が公開する HWND を安全な境界で受け取り、物理座標配置を Win32 に統一。タイトル検索による HWND 特定を避ける |

serde / toml / uuid / rfd / tray-icon は Phase 2/3 の機能を実装する時に追加する。
Phase 1 の固定項目に ID・永続化を持ち込まない。

## API 調査

- [eframe App](https://docs.rs/eframe/0.36.2/eframe/trait.App.html): 非表示中は描画が走らないが `logic` は `request_repaint` に応答する。
- [GetCursorPos](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getcursorpos): screen 座標とエラーを取得。ロック中等の失敗を記録して再試行する。
- 描画用の egui points と screen の物理ピクセルを区別する。配置・範囲取得は SetWindowPos / GetWindowRect、サイズ変換は platform の DPI 関数に集約する。

## 検証結果

環境: stable-x86_64-pc-windows-msvc、rustc/cargo 1.95.0、OSVersion 10.0.26200.0。WMI の OS 表示名取得は sandbox 内で Access denied だったため、OS の build のみを記録した。

実行結果:

- eframe 接続前に `rustc --edition 2024 --test src/core/mod.rs` で純粋な core の 7 tests 成功。
- `cargo fmt --check`、`cargo clippy -- -D warnings`、`cargo test` 成功。7 tests、失敗なし。
- `cargo build --locked` 成功。
- `cargo run --locked -- --smoke-test` は実ウィンドウの 30 回 hide/show と正常終了を確認（終了コード 0、NATIVE_SMOKE_PASS）。
- 追加検査: hidden 時の GetCursorPos 成功回数、復帰後の描画回数、GetWindowRect と選択辺 anchor の一致を smoke に追加。`cargo run --locked -- --smoke-test --edge <edge>` を四辺に対して実施し、すべて終了コード 0。
- 作業スキルは quick_validate.py で検証成功。

手動の Esc、ホットゾーン進入、3 項目クリック、100%/150%、タスクバー・スリープ・ロック・フルスクリーンは未検証。疑似進入で実 HWND が再表示できたことを、実際のカーソルによる画面端復帰の手動合格とは扱わない。

| edge | 実 HWND hide/show | hidden GetCursorPos 成功 | 復帰を含む描画 frame | anchor / 終了 |
| --- | --- | --- | --- | --- |
| Bottom | 30 回 | 68 | 631 | 一致 / 正常 |
| Top | 30 回 | 60 | 493 | 一致 / 正常 |
| Left | 30 回 | 60 | 493 | 一致 / 正常 |
| Right | 30 回 | 60 | 496 | 一致 / 正常 |

30 回ずつの実 hide/show（計 120 回）で復帰不能は発生しなかった。wake と native 操作だけで hidden の logic が進み、shown の UI 描画が再開した。

手動確認手順: `cargo run --locked -- --edge bottom` で起動 → Dock をクリックしてフォーカスを与え Esc → 下端 2 px へカーソルを移す → Dock 外へ出て非表示を待つ。この実操作を 30 回反復し、Explorer / Home / GitHub のクリックを確認する。残る三辺と 100% / 150% でも同様に確認して acceptance の未チェックを更新する。

変更ファイル: Cargo.toml / Cargo.lock / rust-toolchain.toml、src/main.rs / lib.rs / core/{mod,geometry,visibility}.rs / platform_windows/mod.rs / ui/mod.rs、README.md、docs/notes.md / acceptance.md、AGENTS.md、.agents/skills/maximdock-development/{SKILL.md,agents/openai.yaml}。既存の brief と .gitignore は変更していない。

## eframe/winit と Win32 の決定

eframe / egui 0.36.2、winit 0.30.13（Cargo.lock）。Glow renderer を使用。独自イベントループへの置換はしていない。

hidden では eframe の `ui` が実行されないが `logic` は request_repaint に応答する。75 ms の background wake は hidden の時だけ repaint を要求し、GetCursorPos 自体は UI thread の logic から呼ぶ。visible では egui の pointer 入力を使い、hold/delay は単調時刻の deadline で再評価する。終了時に stop / unpark / join で wake thread を止める。

ShowWindow(SW_HIDE / SW_SHOWNOACTIVATE) を双方の可視性操作に使う。winit の set_visible は内部 WindowFlags を持ち、反復表示で SW_SHOW に変わり得るため native show と ViewportCommand::Visible を混ぜない。winit の is_visible は実際の IsWindowVisible を参照するため、eframe の logic-only 分岐は native hide を検知する。位置を計算してから表示し、SetWindowPos に SWP_NOACTIVATE を使う。Esc は Dock にフォーカスがある時だけ有効で、グローバル入力を捕捉しない。

viewport は透明・ボーダーレス・常に最前面・タスクバー非表示・サイズ固定。インタラクティブ Dock に WS_EX_TRANSPARENT は使わない。クリック時の通常フォーカス取得は許可する。

Win32 API:

| API | 選定理由 |
| --- | --- |
| GetCursorPos | hidden のウィンドウイベントに依存しない画面座標取得 |
| MonitorFromPoint / GetMonitorInfoW | Phase 1 の主モニター矩形を取得。rcWork ではなく rcMonitor で実画面端を判定 |
| GetDpiForWindow / SetWindowPos | points → physical px 変換と物理配置を platform に集約 |
| ShowWindow / IsWindowVisible / GetWindowRect | 実 hide/show、復帰後の検証。ShowWindow の戻り値は以前の可視性であって失敗コードではない |
| SHGetFolderPathW(CSIDL_PROFILE) | ユーザー名を埋め込まず現在ユーザー Home を得る |
| ShellExecuteW(open) | exe / lnk / フォルダ / 関連付けファイル / http(s) を Shell で統一。UTF-16 NUL 終端、戻り値 <=32 を型付きエラーにする |

座標は cursor / monitor / anchor が物理 px、egui は points。zoom_factor は初期値 1、native DPI / 96 を window adapter でサイズへ適用。混在 DPI のモニター移行は Phase 3 で調査する。DPI 変更中の再配置や任意 UI zoom の補正は未実装。Win32 の失敗はログへ原因を保持し、再試行または短い UI エラーを示す。ログ対象はファイル basename、URL は query/fragment を除く。Home のフルパスやファイル内容はログに出さない。

## Phase 1 終了時の Phase 2/3 状況（履歴）

| Phase | 対応済み基盤 | 未実装 | 理由 |
| --- | --- | --- | --- |
| 2 | 四辺 CLI・orientation・タイミングの bounds 検証・パス/URL 基本検証 | TOML 読書き、version/UUID モデル、破損時 fallback、設定 UI、追加/削除/rename/順序、rfd、fallback アイコン、保存・可視 edge 変更 | Phase 1 の hidden-window 復帰が最優先。保存破損防止と編集経路の検証は Phase 2 で一体として行う |
| 3 | Win32 adapter、非アクティブ show、primary DPI 変換、短い起動エラー、負座標 core テスト | tray、複数モニター、mixed DPI、外部 DnD、Shell icon/cache、エラー UI 改善、ログイン起動調査 | tray/event loop 共存、DPI 移行、永続化への DnD 接続、GDI リソース所有権の検証を Phase 1 に混在させない |

トレイ: 未採用。Phase 3 で tray-icon の Windows message loop と eframe 共存を調べ、初期化失敗を Dock 本体の致命エラーにしない。設定画面を閉じても終了しない設計が必要。

アイコン: Phase 1 時点は文字ボタン。HICON / GDI リソースを取得していない。Phase 2 で安定した fallback、Phase 3 で最小の Shell 抽出方式を選定し、HICON の DestroyIcon と bitmap の解放、キャッシュ、失敗 fallback を検証する。

Phase 2 推奨順: version 付き serde/toml モデルと検証 → 元ファイル保持と atomic replace を備えた保存 → 編集・順序・重複検出 → 四辺設定と即再配置 → rfd/URL 追加 → fallback アイコン。Phase 3 の DnD はこの保存経路を再利用する。

## Phase 2 実装と検証（2026-10-02）

実装: Top 既定値、version 1 TOML、UUID/IconSource/順序付き項目、設定 child viewport、項目追加/名前編集/順序/削除、四辺設定、タイミング/表示設定、ネイティブファイル/フォルダ選択、fallback アイコン、即時再配置。不正設定・保存失敗は元ファイルを保持して可視の設定画面に通知する。

設定 I/O は platform の ConfigStore に集約。SHGetFolderPathW(CSIDL_APPDATA) で保存先を取得し、同一ディレクトリの create_new 一時ファイルに書込み → sync_all → MoveFileExW(REPLACE_EXISTING | WRITE_THROUGH) で置換する。元ファイルを remove してから rename しない。読み込んだ bytes と保存前の bytes を比較して外部編集を検出する。この比較は通常の競合検出であり、別プロセスと厳密な排他ロックを共有する方式ではない。

不正設定は自動上書きせず save をブロックする。明示 UI 操作でバイト同一の UUID 名バックアップを作り、その後の Apply で置換を許可する。未対応 version/未知フィールドも fallback し、黙って設定を捨てない。保存済み missing path は取り外したドライブを想定して保持し、追加・起動時に存在確認する。

Shell 起動、保存、ダイアログは drawing が発行する command を logic 側で処理する。rfd の同期 picker は専用 thread で動かし、mpsc + request_repaint で結果を UI thread へ返す。UI thread の hidden cursor polling を止めない。rfd の picker API は失敗と取消を詳細な Result で区別しないため、手入力追加も提供する。設定や HWND を worker に持たせない。

設定 viewport は show_viewport_immediate を利用。root が hidden でも child が可視なら eframe が UI を更新することを native smoke で確認した。設定を閉じる時は child を描画対象から外し、root Close を発行しない。UI zoom はキーボードで変更不可にし、DPI 単位のずれを防ぐ。Windows の Meiryo / Yu Gothic を optional fallback として読込み、日本語項目の表示に利用する。フォントを配布しない。

検証:

- `cargo fmt --check` / `cargo clippy -- -D warnings` / `cargo test`: 成功、15 tests。
- config tests: 四辺 TOML round-trip、ID/順序、NaN/タイミング/不正 URL/version/重複、UTF-8/TOML 不正保持、明示バックアップ、Unicode 保存先、外部編集、置換失敗で元ファイル保持と temp cleanup。
- `cargo run --locked -- --smoke-test`: 最終コードで終了コード 0。Top 起動、実 hide/show 30 回（四辺を切替）、hidden GetCursorPos 91 回、描画 655 frames。NATIVE_SMOKE_PASS / PHASE2_SMOKE_PASS。
- native command driver で rename/reorder/add/delete と Apply/save/reload、アイコンサイズ/auto hide/最前面、四辺の即時再配置を確認。非表示中の Apply はウィンドウを表示・移動せず、次回 show で新辺を適用することも検査。hidden root で設定が描画され、child を閉じた後も root が継続する。
- smoke は tempfile 配下の独立設定を使い、終了時に回収する。実 AppData の設定を読み書きしない。

実際のクリック/ネイティブ picker、実カーソル進入、Shell 起動、100%/150% DPI、日本語入力の手動確認は未実施。自動 driver と手動受け入れを混同しない。

Phase 2 終了時の Phase 3 残作業（履歴）: tray/event loop 統合、カーソルモニター選択と表示中固定、mixed DPI、外部 DnD、Shell icon cache/カスタム画像と GDI 所有権、ログイン起動調査。以下の Phase 3 で対応した。

変更ファイル: Cargo.toml/lock、src/core/mod.rs と新規 config.rs、src/main.rs、src/platform_windows/mod.rs と新規 config_store.rs、src/ui/mod.rs と新規 commands.rs/dock_view.rs/settings.rs、README、notes/acceptance、ローカル開発スキル。Phase 1 の幾何・状態機械は維持した。

## 再利用ルール（Phase 2 更新）

`AGENTS.md` と `.agents/skills/maximdock-development/SKILL.md` に、hidden logic、可視性操作の所有、DPI 単位、Win32 戻り値、実機/疑似入力の区別を短く整理。仕様全体は重複せず brief と検証記録を参照する。

## Phase 3 実装（2026-10-03）

実装済み: Windows tray、カーソルモニター選択・表示中固定、対象 HWND の DPI 再取得、外部 dropped-file command と永続化、Shell アイコン worker/cache、指定画像、Reload icons、Discard edits、項目名付き起動失敗メッセージ。既定 Top と既存設定保護・hidden 復帰経路を維持した。

### Tray とイベントループ

tray-icon 0.24.2 / muda 0.19.3 を UI thread で作成・保持。winit 0.30.13 の通常 Win32 message pump と共存し、別 loop は導入しない。[tray-icon の thread 要件](https://docs.rs/tray-icon/0.24.2/tray_icon/) を確認。Show Dock / Settings / Quit の callback は channel へ送信して request_repaint だけを要求し、logic が状態を変更する。

ローカル muda source の handler は OnceCell で、None に設定しても解除できない。一度だけ登録する function relay と Weak callback を使い、終了時に relay の参照を除去し、static に App/egui の強参照を残さない。初期化失敗は警告と設定画面で通知して Dock を継続。Quit は root Close、PollWake は stop/unpark/join、tray は UI thread 上の Drop で回収する。

### モニターと DPI

MonitorFromPoint(MONITOR_DEFAULTTONULL) はカーソルを含むモニターを選び、隙間では None。GetMonitorInfoW の rcMonitor を使用。hidden の hot-zone 進入時だけ候補を更新し、visible 中は HMONITOR ID を固定する。明示 Show も hidden の時だけ再選択。矩形・anchor・実 DPI は debug ログへ記録する。

[GetDpiForWindow](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getdpiforwindow) は現在 HWND のモニター DPI を返す。対象が違う場合、hidden HWND を対象矩形内へ1pxサイズで非アクティブ移動 → 新 DPI 取得 → 最終サイズと anchor を SetWindowPos する。表示中の DPI 変化も logic で補正する。winit dpi.rs は PMv2 を試み PMv1 等へ fallback。points × DPI/96 は platform の physical_size 一箇所だけ、位置は物理 px、egui keyboard zoom は無効。

96/144 DPI のサイズ・負座標 anchor はテスト済みだが、実機100%/150%や混在倍率での移行は未確認。切断で pinned handle の照会に失敗したら通知し hide/show で再選択する。切断・解像度変更への完全自動追従監視は未実装。

### 外部 DnD と設定保護

egui-winit 0.36.2 の WindowEvent::DroppedFile → RawInput.dropped_files、DroppedFile::path() を source で確認。root の drag_and_drop を明示有効化し、イベントを一度消費して DropPaths command を発行。hovered_files がある間は auto hide を保留。複数の既存ファイル／フォルダを入力順で登録し、重複／無効パスは通知する。

正常項目は clone に stage し、一度保存して成功後だけ live config を更新する。破損保護・競合検出は ConfigStore を再利用。未適用 draft があれば全ドロップを拒否し Apply / Discard edits を案内する。URL は明示入力。Dock 内 DnD 並び替えは未実装、Up/Down は維持。command smoke は OS の実 DnD 操作とは区別する。

### アイコンと所有権

[SHGetFileInfoW](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shgetfileinfow) の COM 初期化・background thread・DestroyIcon 要件を採用。worker が CoInitializeEx(APARTMENTTHREADED) / CoUninitialize を対応させる。SHGFI_ICON/LARGEICON の HICON は DestroyIcon、GetIconInfo の color/mask HBITMAP は DeleteObject、CreateCompatibleDC は DeleteDC。RAII で途中エラーでも回収し、cleanup 失敗をログに残す。GetDIBits は未選択 bitmap の top-down 32bit BGRA を読み、alpha が全てゼロの color icon は mask から alpha を復元する。

カスタム PNG/ICO/JPEG は最大4096×4096・decode割当64MiB、texture は最大256px。設定の optional Icon image path または TOML icon.type=file で指定する。画像失敗は Shell、URL は globe、それ以外は fallback。MAX_PATH 超／モノクロ Shell icon は fallback に留める。Shell large icon は低解像度の場合があり、高解像度 image list/thumbnail は未実装。

キャッシュは UUID と target/kind/IconSource。名前・順序・サイズだけでは再抽出せず、変更項目だけ worker に渡す。削除 texture は Drop、古い結果は key 照合で捨てる。Reload icons で同じ画像の内容更新を反映する。描画中に Shell/GDI/デコードを呼ばない。終了は stop/channel を閉じ、停止しない Shell extension 待ちで UI を固めないよう worker は join しない。worker は UI/native window を借用せず、現在の job 後に COM/GDI を解放（停止しない extension はプロセス終了時の OS 回収）。

### 検証・未確認

- fmt / clippy -D warnings / cargo test: 21 tests 成功。追加検証は monitor ID 固定、96/144 DPI/負座標、ドロップ Unicode/order/duplicate/missing/save、cache 不要再読込抑止、custom PNG 優先と欠損 fallback、Shell icon100回の GDI/USER count（warm-up後の増加各+2以下）。
- native smoke: 最終コードで実 HWND30回 hide/show、hidden GetCursorPos92回、描画639 frames、四辺 Apply/save/reload、hidden root の設定、実 tray 作成。疑似 MenuEvent で Settings/Show/Quit、疑似 DropPaths で file/folder 登録・save/reload・duplicate 拒否。終了コード0、NATIVE_SMOKE_PASS / PHASE2_SMOKE_PASS / PHASE3_SMOKE_PASS。
- 実クリック、Explorer DnD、Shell 起動、100%/150% mixed DPI、負座標実機、スリープ/ロック/フルスクリーン、長時間 resource count は未検証。手動受け入れ完了とは扱わない。

手動手順: tray Show → Explorerから複数ファイル／フォルダをドロップ → 再起動復元 → 設定で画像path/Reload icons → Settingsを閉じEscでhide → 別モニターTop端でreveal → 表示中に他画面へ移動して固定を確認。100%/150%、左／上の負座標、四辺で反復し、tray Quitで終了する。

### ログイン時起動の設計調査（登録は未実装）

候補はユーザー単位 HKCU Run と [FOLDERID_Startup](https://learn.microsoft.com/en-us/windows/win32/shell/knownfolderid) のショートカット。今回は OS 設定を変更しない。将来は明示 opt-in、配布exeの絶対パス（引用符付き）、自分の登録だけの解除、exe移動時の修復・二重起動防止を設計する。開発 target/debug exe を自動登録しない。RunOnce は常駐用途に使わない。[Microsoft Run/RunOnce](https://learn.microsoft.com/en-us/windows/win32/setupapi/run-and-runonce-registry-keys) は起動が遅延し得るため即時表示を保証しない。利用者が Explorer でも管理できるユーザー Startup link を第一候補とし、サービスや昇格タスクは不要。登録実装は brief の Phase 4 に留める。
