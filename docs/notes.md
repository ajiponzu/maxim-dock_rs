# 実装・検証記録

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

## Phase 2 と Phase 3 の対応状況・理由

| Phase | 対応済み基盤 | 未実装 | 理由 |
| --- | --- | --- | --- |
| 2 | 四辺 CLI・orientation・タイミングの bounds 検証・パス/URL 基本検証 | TOML 読書き、version/UUID モデル、破損時 fallback、設定 UI、追加/削除/rename/順序、rfd、fallback アイコン、保存・可視 edge 変更 | Phase 1 の hidden-window 復帰が最優先。保存破損防止と編集経路の検証は Phase 2 で一体として行う |
| 3 | Win32 adapter、非アクティブ show、primary DPI 変換、短い起動エラー、負座標 core テスト | tray、複数モニター、mixed DPI、外部 DnD、Shell icon/cache、エラー UI 改善、ログイン起動調査 | tray/event loop 共存、DPI 移行、永続化への DnD 接続、GDI リソース所有権の検証を Phase 1 に混在させない |

トレイ: 未採用。Phase 3 で tray-icon の Windows message loop と eframe 共存を調べ、初期化失敗を Dock 本体の致命エラーにしない。設定画面を閉じても終了しない設計が必要。

アイコン: 現在は文字ボタン。HICON / GDI リソースを取得していない。Phase 2 で安定した fallback、Phase 3 で最小の Shell 抽出方式を選定し、HICON の DestroyIcon と bitmap の解放、キャッシュ、失敗 fallback を検証する。

Phase 2 推奨順: version 付き serde/toml モデルと検証 → 元ファイル保持と atomic replace を備えた保存 → 編集・順序・重複検出 → 四辺設定と即再配置 → rfd/URL 追加 → fallback アイコン。Phase 3 の DnD はこの保存経路を再利用する。

## 再利用ルール

`AGENTS.md` と `.agents/skills/maximdock-development/SKILL.md` に、hidden logic、可視性操作の所有、DPI 単位、Win32 戻り値、実機/疑似入力の区別を短く整理。仕様全体は重複せず brief と検証記録を参照する。
