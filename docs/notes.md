# 実装・検証記録

## 外部ファイルをドロップ先アプリで開く（2026-10-03）

ユーザー指定「アプリで開く。登録は設定から」に従い、Dock全体への外部drop登録を廃止。ui/dock_drop.rsはカード矩形とclipの判定・受け入れ表示・OpenDropped／RejectDrop発行のみ、ui/app/dock_actions.rsが安定UUIDから現在の登録項目を選択しWindows adapterに接続する。platform_windows/file_drop.rsはexe／lnkの実行計画・全ファイル検証・引用・Shell起動を担当。設定のファイル選択は従来のitem_importでdraftに追加しApplyで保存する。外部dropは設定・未保存draftを変更せず、コピー／移動・フォルダーdropも行わない。

lnkはIPersistFile／IShellLinkWでexeと既存引数・作業ディレクトリを取り出す。文書／フォルダーへのlnk、非ファイル、欠損、非Unicode、NUL、長すぎる引数は起動前に拒否。複数ファイルを絶対パスで引用し、一度だけ渡す。独自のcmd／PowerShell経由は使わない。Shell起動は文書に引数を渡さずexeへ渡す方式とする（[ShellExecuteW仕様](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shellexecutew)、[IShellLinkW::GetPath仕様](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-ishelllinkw-getpath)）。COMはscopeで初期化／解放する。追加依存・設定スキーマ変更なし。

OLE drag中はegui pointer更新が届かない場合があるため、Windows adapterでGetCursorPos→ScreenToClient→points変換を行う。外部hover時だけ33msの再描画で対象カードの枠／「このアプリで開く」を更新する。未知位置／clip外／URL／フォルダー／空白へのdropは案内のみ。

fmt／clippy --all-targets -D warnings／43 tests成功。四辺のheadless routingでOpenDroppedは1回、URL／フォルダー／空白／clip外／未知位置はRejectDrop、設定不変を検査。Windowsの実ShellExecuteWで一時ディレクトリにコンパイルした専用receiverをexe／lnk両方から起動し、日本語・空白・&を含む複数パスとlnk既存引数がargvに正確に届くこと、入力ファイル内容不変を検査。ユーザーのアプリは起動しない。Explorerの実OLE操作、混在DPI位置判定、各アプリの引数対応は手動未検証。

native smokeの再実行で、settings初回生成を非表示への遷移と同じpassで行うと、eframe 0.36.2 Glowのimmediate rendererがcallbackを実行できずpanicする経路を再現。利用版sourceでtimed paint時のevent-loop contextとimmediate生成依存を確認し、ui/settings_window.rsへdeferred viewportの接続を分離した。非表示childを一つ準備しておき、logicからVisible commandで開く。Editorのowned snapshotとchannelで編集／commandをAppへ戻し、App強参照・Win32・I/Oをcallbackに入れない。非表示callback無操作と編集結果／Apply commandの受信をテスト。hidden rootから設定を開く回復経路を維持するための修正であり、毎回Dockを表示して代替しない。

修正後native smoke成功: 実hide/show30回、hidden poll92回、689 frames、NATIVE／PHASE2／PHASE3_SMOKE_PASS、終了コード0。並べ替え保存／再読込、未保存draft拒否、意図した競合保存失敗時の順序維持、外部drop拒否時の登録なし、hidden rootでの設定表示を検証。競合テストのERRORログ1回は想定内。設定のdeferred接続は実操作による長時間編集・X閉じ／再開を別途手動確認する。

未実装理由: UWP／Shell namespace／スクリプト／文書カードはアプリとしての引数契約が異なるため対象外。移動済みlnkの自動探索・修復やlnkの起動属性の完全再現はしない。フォルダーへのコピー／移動とDock登録はユーザー指定の対象外。ファイルを本当に開いたかは受け取り先アプリに依存し、Shellの成功だけでは判定できない。Phase 2の設定・保存は維持、Phase 3はこの外部drop起動に仕様変更済み。既存の手動未検証項目は以下の記録を維持する。

## Dock カードのドラッグ並べ替え（2026-10-03）

ユーザー要望によりDock内DnDを実装。egui標準payloadにUUIDを持たせ、カード中心に対する挿入位置を主軸方向で計算して線を表示。release時だけReorder commandを発行し、core::Config::reorder_beforeで安定IDを用いてdraftを作成、既存の検証／競合検出／atomic save成功後に反映する。未保存設定がある場合は拒否。自身／隣接同位置はno-op、削除済みIDは拒否。ドラッグ中はhiddenへ遷移させず、Esc／外へのreleaseは取消。外部ファイルhoverと内部payloadを混同せず、現在の外部dropは上記のアプリ起動に限定する。

責務分離: ui/dock_view/card.rsはカード描画とクリック／drag source、ui/dock_drag.rsは主軸geometryと内部drop feedback／command発行、ui/app/dock_actions.rsは並べ替えstage／保存と外部drop起動接続、coreはID・順序の操作。Appのfieldは公開せず、native／ファイル処理を描画へ入れない。依存・設定スキーマ追加なし。

fmt／clippy --all-targets -D warnings／36 tests成功。四辺の疑似pointer press→move→releaseでcommandが1回だけ発行されLaunchされないこと、Esc／Dock外取消をheadless UIで検査。ID保持・stale ID拒否・TOML往復も追加。native smokeでは並べ替えのApply/save/reload、未保存draft保護、外部編集による保存失敗で順序維持を検証（意図した競合のERRORログが1回出るが成功）。30 hide/show、poll91回、645 frames、3種のPASSと正常終了。OSの実マウスによるドラッグ操作／混在DPIは未検証。

外部drop起動は上記で対応済み。未実装: 長いDockのドラッグ端での自動スクロール・ドラッグ追従ghost（まず確実な順序保存とclick競合防止に限定）。ホイールスクロールと設定のUp/Downを維持。その他のPhase 2/3手動残件は既存記録のまま。

## 高解像度 Shell アイコン（2026-10-03）

縮小表示にはlinear mipmapを有効化。利用版egui_glow 0.36.2のtexture uploadがgenerate_mipmapを呼ぶことをsourceで確認し、256pxから小さいDockサイズへ描画する際のaliasingを抑える。サイズ／DPIに応じた毎回のShell再抽出は追加しない。

フィルター調整後の最終native smoke：30 hide/show、poll91回、652 frames、3種のPASSと正常終了。最終Dock PNGで縮小表示を目視確認。

native smokeも成功：実hide/show30回、hidden poll91回、643 frames、3種のPASSと正常終了。更新後のDock描画PNGを目視確認した。

ユーザー要望により既存の高解像度アイコン残件へ対応。SHCreateItemFromParsingName → IShellItemImageFactory::GetImageで256×256pxを要求し、SIIGBF_ICONONLYでサムネイルを禁止する。SIIGBF_SCALEUPは指定しない。取得・変換失敗（alpha無しを含む）は従来のSHGetFileInfoW + HICON/maskへfallback。ブラウザーexeも同じ取得関数を使用し、ユーザー画像・Builtinの優先順位は変更しない。COM/interface/bitmap/DCはworker内でRAII解放。描画は既存texture cache、DPI変更で再抽出不要。設定スキーマ・依存追加なし。

Image List案と比較し、既存のbitmap変換を再利用でき、Common Controlsの追加featureやv6 manifest管理を不要にするShell Image Factoryを選択。[Microsoft GetImage](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-ishellitemimagefactory-getimage)のICONONLY／リソース解放／worker実行要件を確認。GetImageはDeleteObjectで解放するHBITMAPを返す。BGRA→RGBAとpremultiplied alphaの変換を共有し、半透明エッジのユニットテストを追加。

fmt／clippy --all-targets -D warnings／34 tests成功。実機でexe・フォルダー・関連付け日本語文書は256×256px（従来32×32px）、http／httpsの既定ブラウザーも256×256pxを取得。高解像度優先／失敗時のlegacy呼出／双方失敗、画像優先とURLfallback、100回の実Shell抽出でGDI／USER各+2以下を検証。native resource検査同士はmutexで直列化し、他のnative画像検査の一時ハンドル数が混入しないようにした。

制約: 元リソースが低解像度なら鮮明さは保証できない。すべてのexe／lnk／関連付け製品や混在DPIでの見え方は手動未検証。高解像度の「image list」方式と文書thumbnailは実装しない（代替Factory方式で目的を達成し、Dockはアイコン表示に限定するため）。MAX_PATH超とlegacyモノクロ特殊変換は引き続きfallback。Phase 2/3の他の手動残件、Phase 4の未実装機能は維持。

## URL の既定ブラウザーアイコン（2026-10-03）

更新後のnative smokeも成功：実hide/show30回、hidden poll91回、652 frames、NATIVE／PHASE2／PHASE3_SMOKE_PASSと正常終了。

ユーザー要望により、URL の自動アイコンを地球から既定ブラウザーへ変更。優先順位はユーザー画像 → 明示Builtin → http／httpsの既定ブラウザー → 地球fallback。欠損カスタム画像もブラウザーへfallbackする。URLごとのfavicon取得やWebアクセスは行わない。

Windows層のAssocQueryStringW(ASSOCF_IS_PROTOCOL | ASSOCF_NOTRUNCATE, ASSOCSTR_EXECUTABLE)でプロトコル別の既定実行ファイルを取得し、既存SHGetFileInfoW／GDI変換を再利用。サイズ照会のS_FALSEは成功として扱い、UTF-16バッファを2〜32768文字に制限。COMは同一worker threadでRAII解放する。描画は従来どおりtexture cacheのみ。新しい依存・設定スキーマ・レジストリ書込みなし。API仕様は [Microsoft AssocQueryStringW](https://learn.microsoft.com/en-us/windows/win32/api/shlwapi/nf-shlwapi-assocquerystringw) と [ASSOCSTR](https://learn.microsoft.com/en-us/windows/win32/api/shlwapi/ne-shlwapi-assocstr) を確認した。

fmt／clippy --all-targets -D warnings／31 tests成功。実Windows関連付けからhttp／httpsのブラウザーアイコンを取得し、そのexeのShell画像と寸法・RGBAが一致することを検証。ユーザー画像優先、欠損画像からブラウザー、関連付け失敗時の地球fallback、明示Builtinの維持もテスト。環境の既定ブラウザーを変更せず検証した。

制約: 既定ブラウザー変更は「アイコンを再読み込み」または再起動で反映（自動監視は追加しない）。実行ファイルとして解決できない関連付け、MAX_PATH超、既存Shell変換の非対応アイコンは地球に戻る。ブラウザー変更の実操作・異なるブラウザー製品の網羅は未検証。

## カラーテーマ（2026-10-03）

カラーコード欄のクリックでもピッカーを開くよう改善。コードをボタン表示とし、ポップアップ内にピッカーと直接入力欄を併設。変更はdraftだけに反映する。疑似マウス押下／解放のheadlessテストでポップアップが開き、開くだけでは色が変更されないことを確認。fmt／clippy --all-targets -D warnings／29 tests成功。今回の変更後にnative smokeは再実行しておらず、実マウスのピッカー操作は未検証。

選択文字のコントラスト調整後にも native smoke を再実行し、30 hide/show、poll91回、649 frames、3種のPASSと正常終了を確認。最終PNGでも選択文字とカスタム編集欄を目視確認した。

ユーザー要望により Phase 4 のテーマ統合だけを追加。標準／Ocean／Forest／Rose を明暗モードごとに解決し、カスタムは9色の固定パレットを使用する。色ピッカー、#RRGGBB入力、プリセットからコピー、draft のプレビューを設定へ追加。Dock と設定は共通の style を参照し、適用は既存の検証・保存成功後のみ。起動時にも保存済みのパレットを反映する。CSS は brief の技術制約に従い採用せず、TOML の appearance.palette / appearance.colors を追加。依存追加なし。version 1 の旧 appearance は serde default で標準へ補完し、項目／UUID／順序を保持。色構文は core で検証する。

検証: 28 tests、fmt、clippy --all-targets -D warnings 成功。旧設定の互換読込、全パレットのTOML往復、不正色・未知名拒否、両テーマstyleへの反映・カスタム色から標準へ復帰、全パレット／両明暗／各ページの最小サイズheadless描画を検査。native smoke で5パレットを切替えてApply/save/reload、実hide/show30回、hidden poll91回、655 frames、3種のPASSと正常終了。target/theme-review のカスタム設定rendererを目視確認。実クリック・ピッカー操作・混在DPIは未検証。

制約: カスタムは明暗共通の固定色、コントラスト自動補正なし。CSS・ファイル監視・外部テーマファイルのインポートは未実装。安定した既存TOML保存経路へ限定し、追加のパーサー／監視threadを持ち込まないため。直接編集は終了後に行い再起動する。詳細は color-themes.md。その他の Phase 2/3 の未検証項目と Phase 4 未実装機能は維持する。

## UI 更新（2026-10-03）

ユーザー要望に合わせ、Dock と設定画面の読みやすさ・余白・配色を更新。本文とボタン17 pt、Dock ラベル16 pt、補足15 pt。DPI の換算境界と zoom=1 は維持。Dock は角丸パネル、ホバー／フォーカス強調、大きめのアイコンと歯車設定ボタン。ラベルは固定文字数の切捨てから実幅の省略表示へ変更し、四辺のサイズ計算も共有したタイル寸法から行う。

設定は「Dock／アイテム／詳細」のナビゲーションとカード構成、常時見える保存／破棄フッター、未保存状態表示を追加。初期880×760 pt、最小680×520 pt、本文は縦スクロール。入力欄は最低36 pt高、狭いカードではスライダー幅を縮める。共通配色・文字サイズは ui/theme.rs、設定各ページは ui/settings/ に分離。既存開発 skill に従い描画は command 発行だけとし、保存・Shell・ダイアログの境界は変更しない。依存追加・設定スキーマ変更なし。エッジの再表示判定はユーザー指示により今回の変更対象外。

検証: fmt / clippy -D warnings / 26 tests 成功。両テーマの文字サイズ、四辺・24/56/128 ptアイコンの寸法、最小ウィンドウで全ページの headless 描画（設定や command の不意の変更なし）を追加検証。最終 native smoke は実 hide/show30回、hidden poll92回、645 frames、NATIVE / PHASE2 / PHASE3_SMOKE_PASS と正常終了。

任意の MAXIMDOCK_SMOKE_CAPTURE_DIR を指定すると一時設定の smoke で Dock と設定3ページの PNG を生成する。PNG 書込みは logic 側だけで行い、通常起動では無効。Glow 0.36.2 の immediate child 描画経路は Screenshot action の返却を処理しないため、child 共存の smoke 検査後、同一設定 renderer を確認専用の root に描画して取得する。これは child HWND そのもののスクリーンショットではない。target/ui-review の実描画で日本語・カード・フッター・入力欄を目視確認。実クリック／スクロール／混在 DPI は未検証のまま。

Phase 2/3 の実装状況・残機能は下記の履歴と acceptance を維持。今回 Phase 4 の拡大アニメーションや OS スタートアップ登録は追加しない。UI 操作性の更新に限定し、非表示時の軽量ポーリングと既存の機能境界を維持するため。

## UI 責務分離（2026-10-03）

肥大化した ui/mod.rs から責務を分離した。公開入口 `ui::DockApp`、CLI、設定スキーマ、Top 既定値、hide/reveal と保存の挙動は維持。依存追加なし。

| モジュール | 責務 |
| --- | --- |
| ui/mod.rs | モジュール宣言と DockApp の再公開だけ |
| ui/app.rs | App の所有状態、初期化、eframe logic/ui、表示状態とビューの接続 |
| ui/app/actions.rs | UI command の実行、保存・起動・トレイ／選択結果の接続 |
| ui/app/smoke.rs | native smoke の検証シナリオと計測。通常動作とは分離 |
| ui/poll_wake.rs | hidden-only repaint wake、atomic state、thread の停止／join |
| ui/item_import.rs | 設定のファイル選択項目の検証・順序付き draft 作成とそのテスト |
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

### 外部 DnD と設定保護（当時の登録方式・履歴）

以下の登録方式は上記のユーザー指定で廃止。現在はDockのアプリカードで開き、登録は設定から行う。並べ替えは上記で実装済み。

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
