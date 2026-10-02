# 受け入れ状況

記録日: 2026-10-02。チェックは実際に確認した条件のみ。未チェックの条件には、実装済みだが手動未検証のものも含む。

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

範囲内の機能実装は完了。チェック済みは記載の自動テストや native command smoke の証拠に基づく。smoke は UI コマンド経路を実行するが人間のクリックやダイアログ操作の代用ではない。Shell アイコン抽出/外部 DnD/トレイ/マルチモニターは Phase 3 に残す。

## Phase 3 — 未実装（一部基盤のみ）

- [ ] トレイから Dock 表示・設定・正常終了（現在は Dock 右クリック Quit と smoke 自動終了）
- [ ] カーソルのあるモニターの選択 edge から復帰（主モニターのみ）
- [x] 表示中に別モニターへ追従するコードはなく、配置は起動・復帰時に固定
- [x] 負の仮想座標の core 計算でクラッシュしない（ユニットテスト）
- [ ] 負座標・混在 DPI のマルチモニター実機検証
- [ ] 外部ファイル／フォルダ DnD と永続的な項目追加
- [x] 全項目に安定したコード描画 fallback、URL は globe（Shell 抽出/カスタム画像は未実装）
- [ ] Shell アイコン読込で icon/GDI リソースリークがないことを確認（抽出は未実装）
- [x] 起動失敗は操作可能な設定 viewport と詳細ログへ表示（高度な修復は未実装）
- [ ] ログイン時起動の設計調査（実装は必須でない）

理由: Phase 2 の設定保存と操作を先に安定させる範囲に従った。通知領域はイベントループ統合、複数モニターはモニター固定と DPI 移行、DnD は今回の保存経路への接続、Shell アイコンは所有権・キャッシュ・リソース解放の検証が必要。アイコン抽出を行っていないため、GDI leak の合格を推測で付けない。
