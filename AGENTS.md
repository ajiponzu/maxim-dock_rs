# MaXImDock 開発規則

実装・レビュー時は `docs/codex-implementation-brief.md` を通読し、今回のフェーズ範囲を確認する。継続作業には `.agents/skills/maximdock-development/SKILL.md` を利用する。

- `src/core` は GUI/Win32 非依存、`src/platform_windows` だけに unsafe と Win32 を限定する。`src/ui` は描画と接続を担当する。
- hidden で描画が止まっても復帰できる経路を維持する。変更時は core テストに加えて native smoke を再実施する。
- 依存追加前に目的・代替案を `docs/notes.md` へ記録する。
- fmt / clippy（warnings をエラー）/ test を実施し、実機確認・疑似入力・未検証を `docs/acceptance.md` で区別する。
- 次フェーズの未実装項目と理由を notes / acceptance / README に反映する。入力フックや別イベントループを推測で導入しない。
- Phase 3: モニター再選択は hidden 時だけ。移動前 HWND の DPI を使わない。DnD は未保存 draft と保存競合を保護する。Shell/GDI 抽出は worker と RAII、描画は cache のみ。tray handler の解除可否は利用版 source を確認し、static に App の強参照を残さない。
- ui/mod.rs は公開入口だけに保つ。App 接続・command 実行・smoke を分離し、native dialog の worker/受信状態は platform に置く。ファイル分割のために App field や atomic を広く公開しない。
- 作業環境は core のレシピ／配置数学、platform のモニター catalog・起動・window 操作・worker、UI の draft 編集／command を分離する。起動前の全 top-level HWND/PID を snapshot し、既存・曖昧な window を移動しない。worker の中止は将来の起動／待機だけを止め、利用者のアプリを終了しない。モニターの一時 handle や列挙番号を保存しない。
- UI 配色・文字サイズは ui/theme.rs に集約し、両テーマと最小設定サイズを検証する。DPI のため zoom を増やさず、Dock 寸法は描画と同じタイル寸法から計算する。任意の描画確認 PNG と実操作の受け入れは区別する。
- Dock の寸法はアプリ一覧を基準にする。作業環境の件数や表示モード切替ではリサイズ／再配置せず、多い項目は同じ領域内でスクロールする。
- 作業環境カードの起動時は保存済みレシピと該当draftを比較し、変更・削除が未保存なら旧レシピを起動しない。WSLの既定は標準Linuxシェル経由。コマンド／各引数をcoreでPOSIX引用し、WSLへ渡すその引用済み行だけraw_argで渡す（Windowsの自動引用を重ねない）。直接--execは明示選択とする。
- 設定／切替ボタンはカードの ScrollArea 外の固定領域に置き、両モードで同じ座標に保つ。カードの clip と drop 判定を操作領域へはみ出させない。
- WSL モードのコマンド／cwd をWindowsのexe／directory検査に通さない。System32/wsl.exeへargvを渡し、任意のログインシェルでもコマンド／引数を位置引数として保持する。WSL終了待機とWindows window配置を分離し、配置時は明示したWindows所有者exe・起動前HWND/PIDを使う。診断出力は上限付き・非ブロッキング、EOF待ちや出力の自動ログ保存をしない。中止／timeoutでLinuxプロセスやWSLを強制終了しない。旧設定はWindowsモードを既定に保つ。
- Dock DnDは安定UUIDのpayloadとrelease時のcommandで処理する。描画中に順序やファイルを変更せず、未保存draft／保存競合を保護する。並べ替えでLaunchを発行しないこと、Esc／外への取消を四辺で検証する。外部dropは登録ではなくアプリカードでファイルを開く。exe／exeを指すlnkに限定し、引数を引用して全件検証後に一度起動する。URL／フォルダー／空白には登録・コピー・移動しない。OLEでUI pointerが欠ける場合の位置取得／DPI変換はplatformに置く。
- 設定windowはdeferred viewportのsnapshot／command受信で接続する。callbackにAppの強参照を渡さず、非表示childを準備してlogicから開ける経路を維持する。hidden／timed paint中の初回immediate生成に戻さない。
