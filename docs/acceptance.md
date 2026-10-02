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

## Phase 2 — 未実装（一部基盤のみ）

- [ ] 設定を再起動後も復元する
- [ ] 選択 edge を保存してホットゾーン検知に使用する（CLI 指定の検知のみ対応）
- [ ] 不正 TOML から安全に復帰し、元ファイルを保持する
- [ ] 項目を追加、リネーム、並び替え、削除する
- [ ] 設定 UI で四辺を選ぶ（CLI `--edge` のみ対応）
- [x] Top / Bottom の横配置、Left / Right の縦配置コードと四辺の純粋な配置テスト
- [ ] Top / Bottom / Left / Right の実操作受け入れ確認
- [ ] URL とローカルパスを UI から作成する
- [ ] タイミング・アイコンサイズ・表示設定を保存する
- [ ] fallback アイコン、edge 変更時の即時 UI 再配置

理由: 最優先の hidden-window 復帰 PoC を先に成立させる Phase 1 の範囲に従った。現在は固定 3 項目と検証済み default timing のみ。TOML、編集 UI、ファイルダイアログを途中まで導入すると破損ファイル保持や保存経路を検証できないため、Phase 2 でまとめて実装する。

## Phase 3 — 未実装（一部基盤のみ）

- [ ] トレイから Dock 表示・設定・正常終了（現在は Dock 右クリック Quit と smoke 自動終了）
- [ ] カーソルのあるモニターの選択 edge から復帰（主モニターのみ）
- [x] 表示中に別モニターへ追従するコードはなく、配置は起動・復帰時に固定
- [x] 負の仮想座標の core 計算でクラッシュしない（ユニットテスト）
- [ ] 負座標・混在 DPI のマルチモニター実機検証
- [ ] 外部ファイル／フォルダ DnD と永続的な項目追加
- [ ] exe / lnk / フォルダ / URL の適切なアイコンまたは fallback
- [ ] Shell アイコン読込で icon/GDI リソースリークがないことを確認（抽出は未実装）
- [ ] 起動失敗 UI の改善（現在は短いメッセージと詳細ログのみ）
- [ ] ログイン時起動の設計調査（実装は必須でない）

理由: 通知領域はイベントループ統合、複数モニターはモニター固定と DPI 移行、DnD は Phase 2 の保存・重複検出、Shell アイコンは所有権・キャッシュ・リソース解放の検証がそれぞれ必要。これらは Phase 1 の復帰検証後に順に扱う。アイコン抽出を行っていないため、GDI leak の合格を推測で付けない。
