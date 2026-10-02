# MaXImDock 開発規則

実装・レビュー時は `docs/codex-implementation-brief.md` を通読し、今回のフェーズ範囲を確認する。継続作業には `.agents/skills/maximdock-development/SKILL.md` を利用する。

- `src/core` は GUI/Win32 非依存、`src/platform_windows` だけに unsafe と Win32 を限定する。`src/ui` は描画と接続を担当する。
- hidden で描画が止まっても復帰できる経路を維持する。変更時は core テストに加えて native smoke を再実施する。
- 依存追加前に目的・代替案を `docs/notes.md` へ記録する。
- fmt / clippy（warnings をエラー）/ test を実施し、実機確認・疑似入力・未検証を `docs/acceptance.md` で区別する。
- 次フェーズの未実装項目と理由を notes / acceptance / README に反映する。入力フックや別イベントループを推測で導入しない。
