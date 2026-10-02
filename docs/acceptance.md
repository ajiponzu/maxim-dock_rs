# 受け入れ状況

記録日: 2026-10-03。チェックは実際に確認した条件のみ。未チェックの条件には、実装済みだが手動未検証のものも含む。

## 外部ファイルをアプリで開く（2026-10-03）

- [x] アプリカードの受け入れ表示とOpenDropped command。登録／保存／コピー／移動しない
- [x] 四辺headless routingで1回だけ発行、URL／フォルダー／空白／clip外／未知位置は案内、設定不変
- [x] exeとexeへのlnk、既存引数／作業ディレクトリの取得、複数ファイルの全件検証・引用
- [x] 実Shell起動で専用テストreceiverへ日本語・空白・&を含む複数パスとlnk引数が正確に届くことを検査。入力内容不変
- [x] fmt／clippy --all-targets -D warnings／43 tests成功
- [x] native smoke正常終了: hide/show30回、poll92回、689 frames、3種のPASS。並べ替え保存／draft・競合保護／外部drop登録なし／hidden root設定表示
- [x] settingsをdeferred viewportへ分離し、初回生成時panicを修正。snapshot／command受信と非表示callback無操作をテスト
- [ ] Explorerから実際のアプリカードへdropし、目的のアプリでファイルが開くことを確認
- [ ] OLE hoverの受け入れ表示、複数モニター混在DPIでの実drop位置を確認
- [ ] 遅延方式の設定画面で長時間編集、Xで閉じてtrayから再開する実操作を確認

登録は設定のファイル選択／手入力を維持。UWP・スクリプト・文書カードは対象外、アプリ側がファイル引数に対応する必要がある。lnk自動修復／起動属性の完全再現は未実装。Phase 2保存安全性とPhase 3トレイ／モニター／アイコンは維持。下記の旧「drop登録」検証は履歴で、現在の仕様ではない。

## Dock カードのドラッグ並べ替え（2026-10-03）

- [x] 内部UUID payload、四辺の主軸挿入線、release時に保存するReorder command
- [x] 四辺のheadless UI疑似ドラッグで並べ替え1回だけ発行、Launchなし、Esc／外へのdropで取消
- [x] ID・項目内容維持、同位置no-op、stale ID拒否、TOML往復をテスト
- [x] native command経路で並べ替え保存／再読込、未保存draft拒否、競合による保存失敗時のlive順序維持を検査
- [x] fmt／clippy／36 tests、native smoke正常終了（30 hide/show、poll91回、645 frames、3種のPASS）
- [ ] 実マウスでの長いDock・四辺・混在DPIの並べ替えを手動検証

外部dropは上記のアプリ起動へ変更済み。ドラッグ端の自動スクロール／ghostは未実装、設定のUp/Downを代替に維持。コピー／移動は実行しない。

## 高解像度 Shell アイコン（2026-10-03）

- [x] Shell Image Factoryで256pxを要求。サムネイルは使わず、失敗時はlegacy Shellアイコンへ戻る
- [x] 実exe・フォルダー・関連付け文書・既定ブラウザーで256×256pxの取得を検証（従来exe等は32×32px）
- [x] 高解像度優先／legacy fallback、BGRA・半透明alpha変換をテスト。既存のユーザー画像優先・URL fallbackは維持
- [x] 100回のShell抽出でGDI／USERハンドル増加各+2以下。fmt／clippy --all-targets -D warnings／34 tests成功
- [x] native smoke：実hide/show30回、poll91回、643 frames、3種のPASSと正常終了。Dock描画PNGを目視確認
- [x] linear mipmap縮小フィルターを追加し、最終native smokeも正常終了（30 hide/show、poll91回、652 frames、3種のPASS）。最終Dock PNGを目視確認
- [ ] 多様なexe／lnk／関連付け製品・100%／150%混在DPIでの実表示の手動検証

高解像度アイコンの残件はFactory方式で対応。Image List方式そのものやthumbnail表示は採用しない。元画像の細部やMAX_PATH／モノクロ特殊対応の制約はnotesを参照。

## URL の既定ブラウザーアイコン（2026-10-03）

- [x] URLの自動アイコンはプロトコル別の既定ブラウザーを使用。ユーザー画像・明示Builtinを優先、失敗時は地球fallback
- [x] 実Windowsのhttp／https関連付けを照会し、ブラウザーexeのShell画像と一致することをテスト
- [x] カスタム優先、欠損画像fallback、照会失敗fallback、Builtin維持をテスト。fmt／clippy／31 tests成功
- [x] native smoke：hide/show30回、poll91回、652 frames、3種のPASSと正常終了
- [ ] 実際の既定ブラウザー変更後、再読み込みで切り替わることの手動確認

関連付けの自動監視は未実装。変更後は「詳細 → アイコンを再読み込み」または再起動を使用する。ブラウザーexeを解決できない関連付け／既存Shell変換非対応は地球fallback。

## カラーテーマ（2026-10-03）

- [x] カラーコードのクリックでピッカーを開く。ポップアップ内で直接入力も可能。疑似押下／解放で開くことと色の不意の変更なしをテスト（29 tests、fmt／clippy成功）

- [x] 標準／Ocean／Forest／Roseとカスタム。Dock／設定へ共通で反映
- [x] 明暗モード別プリセット、カスタム9色、色ピッカー／#RRGGBB入力、プリセットからのコピー、draftプレビューを実装
- [x] 旧設定の互換読込、全パレットの保存形式往復、不正色・未知名拒否をテスト
- [x] 両明暗／全パレット／3ページの最小サイズheadless描画、styleへの色反映と標準への復帰をテスト
- [x] fmt / clippy --all-targets -D warnings / test（28 tests）、native smokeの3種のPASSと正常終了。5パレット切替とApply/save/reloadを実行
- [x] カスタム設定の実描画PNGを確認（同一rendererのrootプレビュー、child HWND撮影ではない）
- [ ] 実ピッカー操作、設定クリック／スクロール／適用・破棄・通常再起動の手動検証
- [ ] 全プリセットの実画面での見え方と混在DPIの手動検証

CSSと外部テーマファイルのインポート、ホットリロード、コントラスト自動補正は未実装。既存TOMLと安全な保存経路を利用し、追加のUI基盤や監視処理を導入しないため。カスタム色は明暗共通の固定色。操作方法は color-themes.md。Phase 4のテーマ統合は今回対応し、アニメーション等の他機能は未実装のまま。

## UI 更新 — 読みやすさと描画確認（2026-10-03）

- [x] Dock の角丸パネル・ホバー／フォーカス強調・歯車設定ボタン・16 ptラベルを実描画 PNG で確認
- [x] 設定を Dock／アイテム／詳細へ分割、本文・ボタン17 pt、入力欄36 pt高、保存／破棄フッターを実描画で確認
- [x] ダーク／ライト双方の文字サイズをテスト。最小680×520 ptで全ページを headless 描画し、不意の設定変更や command 発行がないことを検査
- [x] 四辺と24/56/128 ptアイコンでタイル寸法・frame budgetをテスト。zoom=1 と platform DPI 換算を維持
- [x] fmt / clippy -D warnings / test（26 tests）、最終 native smoke 正常終了。実hide/show30回、poll92回、645 frames、3種の PASS
- [x] optional capture smoke で Dock と設定3ページの PNG を保存して目視確認。設定 PNG は同一 renderer の root プレビューであり、child HWND 自体の撮影ではない
- [ ] 実クリックによるページ切替・編集・適用・破棄、最小サイズでのスクロールとキーボード操作の手動検証
- [ ] ライト配色・100%/150%混在 DPI・長い日本語名の実画面での操作性検証

配色とページ構成は更新したが、Phase 2 の保存安全性、Phase 3 の tray／モニター／Shell／DnD は既存経路を維持。edge 再表示判定は今回変更しない。Phase 4 のアニメーション・スタートアップ登録等の残機能は下記と notes に記載したまま、UI 更新の範囲外として未実装。

## UI 責務分離 — 回帰検証

- [x] ui/mod.rs はモジュール宣言と公開入口だけ。ui::DockApp を維持
- [x] App 接続、command 実行、native smoke、PollWake、項目取り込みを分離
- [x] rfd worker と受信状態を Windows FilePicker に移動。App field/atomic の公開範囲を広げない
- [x] fmt / clippy -D warnings / cargo test（23 tests）
- [x] picker の pending・多重起動拒否・パス順序・取消・切断を追加テスト
- [x] 分離後の native smoke（既存 Dock を維持し別 target-dir で実行）。実 hide/show30回、poll92回、641 frames、3種の PASS と正常終了

既存の手動未検証項目は本リファクタリングで合格へ変更しない。

## Phase 1 — 実装済み・手動受け入れ一部未完了

- [x] Rust stable、egui + eframe、windows crate、Dock 1 ウィンドウ
- [x] core / platform_windows / ui の層分離
- [x] 四辺の orientation・ホットゾーン・境界・負座標・anchor の純粋関数テスト
- [x] Hidden / Revealing / Visible / HidePending、200 ms hold、500 ms delay、戻り・Esc の状態遷移テスト
- [x] ハードコード 3 項目（Explorer / 現在ユーザー Home / GitHub）、ShellExecuteW アダプター
- [x] `cargo fmt --check`
- [x] `cargo clippy -- -D warnings`
- [x] `cargo test`（7 tests）
- [x] Windows ローカルで `cargo run --locked -- --smoke-test` が起動・正常終了（OS build 26200）
- [x] 低レベルグローバルマウスフック未使用、unsafe は Windows binding のみ（crate-level deny + platform 内 allow）
- [x] 本物の HWND を 30 回 hide/show して復帰。疑似ホットゾーン進入を使用
- [x] 四辺それぞれ 30 回（合計 120 回）の native smoke。配置一致・hidden GetCursorPos・復帰後描画・正常終了を確認
- [ ] 通常起動して実際の Esc キー入力で隠れることを手動確認（処理実装済み、フォーカスが必要）
- [ ] 実際に選択辺の 2 px にマウスを置き、非表示 Dock が復帰することを手動確認（GetCursorPos + 判定の接続実装済み）
- [ ] マウスが実 Dock 範囲外へ出た後の 500 ms hide / 200 ms hold を手動確認（状態機械は自動検証済み）
- [ ] Explorer / Home / GitHub をクリックして各対象が開くことを手動確認（Shell アダプター実装済み）
- [ ] 実際の画面端進入で hide/reveal 30 回の手動反復（native smoke は疑似進入のため別判定）
- [ ] 100% / 150% DPI、タスクバー、フォーカス、スリープ・ロック・フルスクリーンを手動確認

## Phase 2 — 実装済み・自動検証済み、手動受け入れ一部未完了

- [x] 新規設定・安全な fallback の既定辺は Top（ユーザー指示）。保存済み edge は尊重
- [x] version/UUID/順序付き TOML 設定の保存・再読込（往復/保存テスト、native Apply/save/reload）
- [x] 保存された edge とタイミングをホットゾーン判定へ接続
- [x] 不正 TOML/UTF-8/設定値から既定値へ fallback、元ファイル保持、保存をロック（テスト）
- [x] 明示的なバイト同一バックアップ後のみ不正ファイルの置換を許可（テスト）
- [x] 同一ディレクトリの一時ファイル・sync・Win32 replace。置換失敗で元ファイルを保持（テスト）
- [x] 外部編集による保存競合を検出し上書きを拒否（テスト）
- [x] 項目の追加/名前編集/Up/Down/削除 UI、安定した ID と重複検出（core/editor テスト、command smoke）
- [x] 設定 UI で Top/Bottom/Left/Right、アイコンサイズ、間隔、自動 hide、タイミング、最前面、不透明度、基本 UI theme を編集
- [x] 横/縦 Dock、コード描画 fallback アイコン、長い Dock のスクロール
- [x] 可視 edge 変更は Apply 後に即再配置、hidden 変更は次回表示で適用（native command smoke）
- [x] URL/path の入力追加 UI、rfd のファイル複数選択/フォルダ追加を接続
- [x] Shell/保存/ダイアログは描画から command 境界へ分離。ダイアログは UI thread を塞がない worker
- [x] hidden root と設定 child viewport の共存、設定を閉じてもアプリは継続（native smoke）
- [x] fmt / clippy -D warnings / test（15 tests）
- [ ] 実際のボタン操作で追加・rename・順序変更・削除・Apply・再起動復元を手動確認
- [ ] ネイティブファイル/フォルダダイアログを実操作し、取消・複数追加・重複拒否を確認
- [ ] 四辺・ホットゾーン・Esc を実マウスとキーで確認
- [ ] 日本語表示/入力、100%/150% DPI、設定ウィンドウの操作性を手動確認

範囲内の機能実装は完了。チェック済みは記載の自動テストや native command smoke の証拠に基づく。smoke は UI コマンド経路を実行するが人間のクリックやダイアログ操作の代用ではない。以下の Phase 3 で統合機能を追加した。

## Phase 3 — 統合実装済み・実操作／混在 DPI の受け入れ未完了

- [x] 実 Windows トレイ作成、メニュー callback → weak relay/channel/repaint → UI command の接続
- [x] 疑似 MenuEvent による hidden root で Settings → Show → Quit・正常終了（native smoke）
- [ ] 実際のトレイメニュー操作から Dock 表示・設定・正常終了を手動確認
- [x] MonitorFromPoint でカーソルモニターを選択し、その矩形の設定 edge へ配置するコードを接続
- [x] 表示中は選択モニターを固定。別 ID の入力は状態機械の固定 ID を変更しない（テスト）
- [ ] 実際の別モニターの画面端から復帰し、表示中に移動しないことを手動確認
- [x] 負の仮想座標の core 計算でクラッシュしない（ユニットテスト）
- [x] 対象モニターへ hidden HWND を移してから window DPI を取得する補正。96/144 DPI のサイズ・負座標 anchor テスト
- [ ] 負座標・混在 DPI のマルチモニター実機検証
- [x] egui 0.36 DroppedFile::path を command 化し、複数ファイル／フォルダ・Unicode・重複拒否・順序・保存を接続
- [x] ドロップ command から Apply/save/reload、重複拒否を native smoke で検証（OS の実ドロップではない）
- [x] 未保存 draft はドロップで上書きせず拒否。Discard edits を追加。保存失敗は live config を変更しない
- 旧drop登録の手動検証は現仕様では不要（ユーザー指定により廃止）。現在のアプリdrop受け入れは上記を参照
- [x] Shell icon worker・キャッシュ・削除時の texture 回収。名前/順序/サイズ変更だけでは再抽出しない（テスト）
- [x] カスタム PNG/ICO/JPEG → Shell → URL globe/fallback。指定画像の優先と欠損 URL 画像 fallback をテスト
- [x] Shell icon 100 回抽出で GDI/USER ハンドル増加が各 +2 以下（warm-up 後、実 Win32 テスト）
- [ ] 多様な exe/lnk/関連付けファイル・モノクロ icon・カスタム画像を反復編集して見た目と長時間 resource 使用を手動確認
- [x] 起動失敗を項目名付きの操作可能な設定 viewport と詳細ログへ表示（高度な修復は未実装）
- [x] ログイン時起動の設計調査を notes に記録。OS へのスタートアップ登録は変更しない
- [x] fmt / clippy -D warnings / test（21 tests）、native smoke の NATIVE / PHASE2 / PHASE3_SMOKE_PASS

未実装: モノクロ Shell icon の特殊変換、MAX_PATH 超の Shell icon 抽出、スタートアップ登録、Phase 4 の拡大等。高解像度は上記Factory方式、Dock内DnD並べ替えは上記UUID方式で対応済み。安定した Up/Down と fallback を残し、複雑な Shell/画像経路と OS 自動起動の変更は後回し。混在 DPI／OS 実入力は自動テストで代替できないため未合格。詳細と検証手順は notes を参照。
