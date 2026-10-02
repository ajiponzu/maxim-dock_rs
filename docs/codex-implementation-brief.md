# MaXImDock v2: Codex Implementation Brief

## 1. Objective

MaXImDock v2 は、Windows 向けの常駐 GUI ランチャーである。

指定した画面端に隠れる Dock を表示し、登録済みのアプリケーション、ショートカット、フォルダ、関連付け済みファイル、URL をクリックして開けるようにする。macOS の Dock に着想を得るが、macOS の完全な模倣ではなく、Windows で軽量かつ実用的に動く Dock ランチャーを目標とする。

アプリケーション全体を Rust で実装する。Dock UI は `egui + eframe`、Windows 固有機能は `windows` crate で実装する。

最初の最優先目的は、次の技術的成立性を確認することである。

> eframe の Dock ウィンドウを完全に hide した後でも、Rust 側で Win32 `GetCursorPos` を定期取得し、画面端のホットゾーンを検知して確実に再表示できること。

中核が成立した後、設定保存、通知領域、ドラッグ＆ドロップ、Shell アイコン抽出、マルチモニターを段階的に実装する。

---

## 2. Product Concept

### 2.1 Primary behavior

- Dock は、設定で指定した画面端に配置される
- Dock は通常、完全に隠れている
- カーソルが指定端のホットゾーンに入ると Dock を表示する
- Dock 上の項目をクリックすると、アプリ、フォルダ、ショートカット、文書、URL を開く
- カーソルが Dock の外に出た後、指定時間が経過すると Dock を隠す
- Dock の表示辺は、下端・上端・左端・右端から選べる
- Dock は表示辺に応じてアイテムの並び方向を変える
  - 下端・上端: 横方向
  - 左端・右端: 縦方向
- Dock は通知領域から明示的に表示、設定、終了できる
- 設定と Dock アイテムはアプリ再起動後も復元される

### 2.2 Target environment

- 主対象: Windows 11
- 実用上の最低対象: Windows 10 以降。ただし依存ライブラリが対応する範囲に従う
- アーキテクチャ: x86_64 Windows
- Rust: stable toolchain
- v2 初期リリースは Windows 専用
- Linux/macOS 対応は不要

---

## 3. Non-goals

初期リリースでは、以下を実装対象に含めない。

- Windows タスクバーの置き換え
- macOS Dock の完全な互換実装
- プロセス管理ツールやタスクマネージャー相当の機能
- グローバルキーロガー
- ポーリングで不足が確認されるまでの低レベルグローバルマウスフック
- クラウド同期、アカウント、複数ユーザー共有
- プラグインマーケットプレイスや外部プラグイン API
- スタートメニューや PowerToys Run 相当の全アプリ検索インデックス
- Docker、MCP、AI エージェント、開発環境オーケストレーション
- Linux/macOS 版の配布

ただし、設定永続化、通知領域、ドラッグ＆ドロップ、アイコン抽出、マルチモニター、Dock の表示辺選択は非目標ではない。これらは必要な v2 機能として設計に含め、段階的に実装する。

---

## 4. Technology Decisions

### 4.1 Required stack

| 領域               | 技術                                   | 方針                                               |
| ------------------ | -------------------------------------- | -------------------------------------------------- |
| 言語               | Rust stable                            | アプリケーションロジックを Rust で実装する         |
| GUI                | `egui` + `eframe`                      | Dock と設定 UI を実装する                          |
| ウィンドウ基盤     | `winit`（`eframe` 経由）               | 必要な場合を除き、独自イベントループに置き換えない |
| Windows API        | `windows` crate                        | Windows 固有コードを platform 層へ閉じ込める       |
| 設定保存           | `serde` + `toml`                       | ユーザー設定を永続化する                           |
| エラー             | `thiserror`、必要により境界で `anyhow` | core/platform は型付きエラーを優先する             |
| ログ               | `tracing` + `tracing-subscriber`       | 起動、設定、起動失敗、Win32 失敗を記録する         |
| ID                 | `uuid`                                 | Dock アイテムへ安定した識別子を付与する            |
| ファイルダイアログ | `rfd` または妥当な代替                 | 手動の項目追加にネイティブダイアログを使う         |
| トレイ             | `tray-icon` または妥当な代替           | eframe/winit と安全に共存させる                    |
| 時刻               | `std::time::Instant`                   | 表示状態の時間遷移に単調時刻を使う                 |

### 4.2 Architectural rules

- Tauri、WebView、JavaScript、TypeScript、Electron、HTML/CSS を使わない
- `unsafe` は Windows platform 層だけに置く
- `windows` crate の API 呼び出しを UI コードへ散らさない
- Shell API を描画関数から直接呼ばない
- ホットゾーン判定と表示状態遷移は、実ウィンドウなしでテスト可能にする
- 画面端検知のためだけに `WH_MOUSE_LL`、`SetWindowsHookExW`、グローバル入力キャプチャを導入しない
- `GetCursorPos` を約 50〜100 ms 間隔でポーリングする。初期値は 75 ms
- 技術上の理由なく eframe/winit のイベントループを置き換えない
- 必須スタック以外の依存 crate を追加する際は、目的と代替案を `docs/notes.md` に記録する

---

## 5. Architecture

ドメインロジック、Windows 統合、GUI を分離する。

```text
maximdock/
├── Cargo.toml
├── crates/
│   ├── maximdock-core/
│   │   ├── config.rs
│   │   ├── dock_item.rs
│   │   ├── dock_edge.rs
│   │   ├── geometry.rs
│   │   ├── visibility.rs
│   │   ├── validation.rs
│   │   ├── launcher.rs
│   │   └── lib.rs
│   │
│   ├── maximdock-platform-windows/
│   │   ├── cursor.rs
│   │   ├── monitor.rs
│   │   ├── dpi.rs
│   │   ├── shell.rs
│   │   ├── icon.rs
│   │   ├── app_data.rs
│   │   ├── window.rs
│   │   ├── tray.rs
│   │   └── lib.rs
│   │
│   └── maximdock-desktop/
│       ├── app.rs
│       ├── dock_view.rs
│       ├── settings_view.rs
│       ├── animation.rs
│       ├── commands.rs
│       └── main.rs
│
├── docs/
│   ├── codex-implementation-brief.md
│   ├── notes.md
│   └── acceptance.md
│
├── README.md
├── rust-toolchain.toml
├── clippy.toml
└── deny.toml
```

初回実装では単一 crate で始めてもよい。ただし、次のモジュール境界を維持する。

```text
core/
platform_windows/
ui/
```

中核機能が検証できた後、保守性が上がる場合にだけ workspace crate へ分割する。

### 5.1 Layer responsibilities

| 層                           | 責務                                                                    | 禁止事項                              |
| ---------------------------- | ----------------------------------------------------------------------- | ------------------------------------- |
| `maximdock-core`             | モデル、設定スキーマ、端の幾何計算、状態機械、ターゲット検証            | Win32 や eframe API の呼び出し        |
| `maximdock-platform-windows` | カーソル、モニター、DPI、Shell 起動、アイコン、アプリデータパス、トレイ | UI 状態の保持、UI 描画                |
| `maximdock-desktop`          | Dock と設定画面の描画、UI イベントの接続                                | `unsafe`、直接的な Win32 API 呼び出し |

---

## 6. Domain Model

### 6.1 Dock edge and layout direction

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DockEdge {
    Bottom,
    Top,
    Left,
    Right,
}

impl DockEdge {
    pub fn is_horizontal(self) -> bool {
        matches!(self, Self::Bottom | Self::Top)
    }

    pub fn is_vertical(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }
}
```

ルール:

- `Bottom` と `Top` は横方向の Dock
- `Left` と `Right` は縦方向の Dock
- 選択した辺ごとにホットゾーン幅を使う
- 選択した辺は TOML に保存する
- 設定画面で四辺すべてを選べる
- 表示中に辺を変更した場合、Dock を即座に再配置する
- Dock が隠れている場合、次回表示時に新しい辺を適用する

### 6.2 Dock item

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockItem {
    pub id: Uuid,
    pub label: String,
    pub target: String,
    pub kind: DockItemKind,
    pub icon: IconSource,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DockItemKind {
    Path,
    Url,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IconSource {
    Auto,
    File { path: PathBuf },
    Builtin { name: String },
}
```

`Path` の対象:

- `.exe` など実行可能ファイル
- `.lnk` ショートカット
- フォルダ
- Windows の関連付けで開くファイル

`Url` の対象:

- `https://`
- `http://`

初期実装で過度に URI スキームを一般化しない。

### 6.3 Visibility state

Dock 表示状態は明示的な状態機械で扱う。

```rust
pub enum DockVisibility {
    Hidden,
    Revealing {
        started_at: Instant,
        monitor_id: MonitorId,
    },
    Visible {
        monitor_id: MonitorId,
    },
    HidePending {
        monitor_id: MonitorId,
        since: Instant,
    },
}
```

状態遷移:

```text
Hidden
  └─ カーソルが設定済みの端のホットゾーンに入る
       → Revealing

Revealing
  └─ 最低表示時間を満たす
       → Visible
  └─ Esc
       → Hidden

Visible
  └─ カーソルが実際の Dock 範囲外へ出る
       → HidePending
  └─ Esc
       → Hidden

HidePending
  └─ カーソルが実際の Dock 範囲へ戻る
       → Visible
  └─ hide_delay が経過する
       → Hidden
  └─ Esc
       → Hidden
```

初期値:

```text
cursor_poll_interval_ms = 75
hot_zone_px = 2
hide_delay_ms = 500
reveal_hold_ms = 200
```

これらは設定として保持し、妥当性検証と安全な上下限を設ける。

### 6.4 Geometry model

Windows のカーソル・モニター座標には物理ピクセルを使う。

```rust
pub struct ScreenPoint {
    pub x: i32,
    pub y: i32,
}

pub struct MonitorRect {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}
```

マルチモニター構成では負の仮想デスクトップ座標を取り得るため、非負と仮定してはならない。

core 層に純粋関数を置く。

```rust
pub fn is_in_hot_zone(
    edge: DockEdge,
    cursor: ScreenPoint,
    monitor: MonitorRect,
    hot_zone_px: i32,
) -> bool;

pub fn dock_anchor_position(
    edge: DockEdge,
    monitor: MonitorRect,
    dock_size_px: (i32, i32),
) -> ScreenPoint;
```

ホットゾーン条件:

| Dock edge | ホットゾーン条件                      |
| --------- | ------------------------------------- |
| Bottom    | モニター下端から `hot_zone_px` 行以内 |
| Top       | モニター上端から `hot_zone_px` 行以内 |
| Left      | モニター左端から `hot_zone_px` 列以内 |
| Right     | モニター右端から `hot_zone_px` 列以内 |

いずれの場合も、カーソルが対象モニターのもう一方の軸の範囲内であることを確認する。

---

## 7. Windows Platform Integration

### 7.1 Cursor position

Dock が隠れている間は、`windows` crate を通じて `GetCursorPos` を使用する。

```rust
pub trait CursorProvider {
    fn screen_position(&self) -> Result<ScreenPoint, PlatformError>;
}
```

要件:

- 仮想デスクトップ上の物理ピクセル座標を取得する
- `unsafe` 呼び出しは小さな内部関数に閉じ込める
- Windows 側の失敗情報を保持する
- 隠れた Dock ウィンドウがポインターイベントを受ける必要がない設計にする
- Dock が隠れている間だけポーリングする
- 初期実装では `WH_MOUSE_LL` や `SetWindowsHookExW` を使わない

### 7.2 Monitor selection

マルチモニターを支援する。

要件:

- 現在のカーソル座標を含むモニターを判定する
- Dock が隠れている時、指定辺のホットゾーンへ入ったモニター上に Dock を出す
- Dock をそのモニターの選択辺に配置する
- Dock が表示中は、カーソルが他モニターへ移動しても勝手に Dock を移動しない
- Dock を隠した後、次回表示時に対象モニターを再判定する
- 主モニターの左側または上側にあるモニターも扱う
- デバッグログにモニター矩形と DPI を出力する

候補 API:

```text
MonitorFromPoint
GetMonitorInfoW
GetDpiForMonitor または適切な代替 API
```

### 7.3 DPI handling

要件:

- Win32 のカーソル・モニター座標を物理ピクセルとして扱う
- 導入した eframe/winit のウィンドウ位置・サイズ API が物理／論理どちらを受け取るか確認する
- 物理座標から論理座標への変換は一箇所に集約する
- CSS 的な論理単位と Win32 の物理ピクセルを混在させない
- 100% と 150% の表示倍率で手動検証する
- フレームワーク固有の DPI 制約は `docs/notes.md` に記録する

### 7.4 Shell launch

実行可能ファイルだけでなく、フォルダ、ショートカット、関連付け済みファイル、URL を統一的に開くため、Windows Shell を利用する。

```rust
pub trait ShellLauncher {
    fn open_target(&self, target: &str) -> Result<(), PlatformError>;
}
```

`ShellExecuteW` または同等の Shell API を利用する。

期待動作:

| 対象                   | 動作                                     |
| ---------------------- | ---------------------------------------- |
| `.exe`                 | アプリケーションを起動                   |
| `.lnk`                 | Windows Shell 経由でリンクを解決して起動 |
| フォルダ               | Explorer で開く                          |
| 関連付け済みファイル   | 既定アプリケーションで開く               |
| `http://` / `https://` | 既定ブラウザで開く                       |
| 存在しないパス         | クラッシュせず、行動可能なエラーを表示   |
| 不正な URL             | クラッシュせず、行動可能なエラーを表示   |

要件:

- UTF-16 変換を正しく行う
- 戻り値を検査し、失敗を適切なエラーへ変換する
- 対象とマスク済みの失敗情報をログに残す
- 初期版では `runas` による昇格起動を実装しない

### 7.5 Icon extraction

アイコン抽出は必要機能として扱う。

優先順位:

```text
1. ユーザー指定のアイコンファイル
2. 対象ファイルに対する Windows Shell アイコン
3. URL 用の組み込みアイコン
4. 組み込み fallback アイコン
```

要件:

- `.exe`、`.lnk`、フォルダ、関連付け済みファイルのアイコン抽出を目標にする
- HICON、ビットマップ変換、GDI リソース解放は Windows platform 層へ閉じ込める
- 抽出結果は `egui::ColorImage` または UI 非依存の画像型へ変換する
- 毎フレーム Shell/GDI 抽出を行わない。キャッシュする
- 抽出失敗時は fallback アイコンで継続する
- GDI リソースをリークしない
- 初期段階で実装量が大きすぎる場合は、安定した fallback アイコンを実装し、残作業を `docs/notes.md` に記録する

調査候補 API:

```text
SHGetFileInfoW
IShellItemImageFactory
DestroyIcon
GetIconInfo
GetDIBits
```

候補を無差別に導入せず、最小で安定する方式を選び、その理由を記録する。

### 7.6 Tray integration

通知領域は必要機能として扱う。

要件:

- Dock が隠れていてもアプリケーションは実行継続できる
- トレイのコンテキストメニューには少なくとも以下を置く
  - Dock を表示
  - 設定を開く
  - 終了
- 終了時はポーリングを止め、プロセスを正常終了する
- 設定画面を閉じても必ずしもアプリを終了しない
- トレイ初期化失敗だけで Dock 本体の起動を失敗させない
- トレイイベントを eframe/winit の UI 状態へ安全に橋渡しする

候補 crate は `tray-icon`。採用する場合、eframe/winit のイベントループと共存できるか確認し、採用理由・代替案を `docs/notes.md` に残す。

### 7.7 Window behavior

Dock ウィンドウの目標:

- ボーダーレス
- 半透明または透明背景
- 常に最前面
- タスクバーに通常アプリとして表示しない
- 非表示時は underlying window の hide API を使う
- 表示時には位置を再計算してから、または再配置と同時に show する
- 不必要にフォーカスを奪わない
- 表示中はクリック可能
- インタラクティブな Dock 自体をクリック透過にしない

eframe の標準設定で不足する場合のみ検討する Win32 拡張スタイル:

```text
WS_EX_TOOLWINDOW
WS_EX_NOACTIVATE
WS_EX_TOPMOST
```

`WS_EX_TRANSPARENT` はクリック透過になるため、インタラクティブ Dock には使わない。

---

## 8. Configuration

### 8.1 Storage path

ユーザーごとのアプリデータディレクトリへ設定を保存する。

推奨 Windows パス:

```text
%APPDATA%\MaXImDock\config.toml
```

ユーザー名をハードコードしてはならない。platform-aware なディレクトリ crate、または小さな Windows platform 関数を使う。

### 8.2 Configuration example

```toml
version = 1

[dock]
edge = "bottom"
icon_size = 56.0
spacing = 10.0
hot_zone_px = 2
hide_delay_ms = 500
reveal_hold_ms = 200
always_on_top = true
auto_hide = true

[appearance]
theme = "system"
background_opacity = 0.86

[[items]]
id = "8ca8a9cb-b522-47e1-a06d-a699c1ff0001"
label = "Visual Studio Code"
target = "C:\\Users\\user\\AppData\\Local\\Programs\\Microsoft VS Code\\Code.exe"
kind = "path"
icon = { type = "auto" }

[[items]]
id = "47420696-e01f-4a5a-9caf-166a2a01b816"
label = "Projects"
target = "C:\\Users\\user\\Projects"
kind = "path"
icon = { type = "auto" }

[[items]]
id = "7b28089b-9a63-4ba4-a2ee-e6984de75580"
label = "GitHub"
target = "[https://github.com/](https://github.com/)"
kind = "url"
icon = { type = "builtin", name = "globe" }
```

### 8.3 Persistence requirements

- Dock edge、タイミング、表示設定、順序付きアイテムを保存する
- 適用前に設定を検証する
- 一時ファイルへの書き込み後、置換または rename する方式で破損リスクを下げる
- 読み込めない、または不正な設定を自動上書きしない
- 設定が不正な場合
  - エラーをログに残す
  - 元ファイルを保持する
  - 安全な既定設定で起動する
  - 設定 UI または可視の箇所でエラーを示す
- バージョンフィールドを持ち、将来の移行を可能にする
- アイテム編集後は適切に保存するが、アニメーションフレームごとには書き込まない

---

## 9. User Interface

### 9.1 Dock layout

- Bottom と Top は横方向
- Left と Right は縦方向
- Dock は対象モニターの選択辺で中央寄せにする
- Dock が対象モニター領域から完全にはみ出さないようにする
- 角丸の半透明フローティングサーフェスを使う
- アイコンと、必要に応じて短いラベルを表示する
- クラシックなメニューバーの見た目にはしない
- 初期は簡潔な見た目でよく、後から調整できる構造にする

### 9.2 Hover magnification

Dock アイコンは、macOS 的な着想を持つ拡大表示を支援する。ただし、ピクセル単位での完全再現は目的にしない。

```rust
fn magnification(distance: f32, radius: f32, max_bonus: f32) -> f32 {
    let t = (1.0 - distance / radius).clamp(0.0, 1.0);
    1.0 + max_bonus * t * t
}
```

初期値:

```text
base_icon_size = 56 px
magnify_radius = 160 px
max_bonus = 0.45
```

要件:

- Dock の主軸で、カーソルとアイコン中心の距離を算出する
  - 横 Dock: X 軸
  - 縦 Dock: Y 軸
- 拡大後の矩形に合わせてクリック領域を再計算する
- 描画範囲とヒットテスト範囲を一致させる
- アニメーション／ホバー変化がある間だけ再描画を要求する
- PoC で中核機能を阻害するなら、最初は単純なホバー強調でもよい

### 9.3 Context menus and settings

利用者が Dock アイテムを管理できる必要がある。

設定 UI の必須操作:

- アイテム一覧
- アイテム追加
- URL 追加
- アイテム削除
- 表示名編集
- 並び替え
- Dock edge 選択: bottom / top / left / right
- アイコンサイズ変更
- 自動非表示の有効／無効
- hide delay と hot-zone 幅の設定
- 設定ファイルの場所を開く
- アプリケーション終了

右クリックメニューの最小構成:

```text
- Dock を表示
- 設定
- 終了
```

アイテム単位のコンテキストメニューは、基本の設定画面が動作した後に追加してよい。

---

## 10. Drag and Drop

外部からファイル／フォルダを Dock へドロップして登録する機能を実装対象に含める。

要件:

- 表示中の Dock にファイルおよびフォルダをドロップできる
- 複数パスの同時ドロップを許可する
- 各対象から `DockItem` を生成する
- ファイル名またはフォルダ名から既定ラベルを生成する
- 同じ target を重複追加しない
- 追加後は設定を永続化する
- 追加できない対象はユーザーに分かるように通知する
- Dock 内の並び替え DnD は後回しでもよいが、データモデル上の順序は保持する

実装順:

1. 「ファイル／フォルダ追加」と「URL 追加」の明示的操作を実装する
2. 使用する eframe/egui が提供する dropped-file イベントを調査して利用する
3. eframe/winit だけで実現できない場合でも、理由を記録せずに別の大きな DnD フレームワークを追加しない

---

## 11. Implementation Phases

### Phase 1: Core technical PoC — required

最小の end-to-end Dock を実装する。

範囲:

- Rust stable
- `egui + eframe`
- `windows` crate
- Dock ウィンドウ 1 個
- 初回コミットでは主モニター対応でよい
- `DockEdge` と四辺の幾何計算は最初から実装する
- 最初の視覚検証は Bottom から始めてもよい
- ハードコードした 3 項目を表示する
  - Explorer: `C:\Windows\explorer.exe`
  - Home: 現在ユーザーのホームディレクトリ
  - GitHub: `https://github.com/`
- Windows Shell API 経由で起動する
- Esc で Dock を hide する
- Dock が hidden の間、75 ms 間隔で `GetCursorPos` を取得する
- 選択された edge の 2 px ホットゾーンで再表示する
- 表示中に Dock 範囲外へ出てから 500 ms 経過で hide する
- 再表示直後 200 ms は自動 hide しない
- 幾何計算と状態機械のユニットテストを書く
- `README.md` と `docs/notes.md` を作る

Phase 1 の受け入れ条件:

- `cargo fmt --check` が通る
- `cargo clippy -- -D warnings` が通る
- `cargo test` が通る
- Windows 11 で `cargo run` により Dock が起動する
- Esc で Dock が隠れる
- Dock を隠した後、選択辺のホットゾーンで再表示される
- Explorer、フォルダ、GitHub を起動できる
- hide/reveal を少なくとも 30 回繰り返しても Dock が復帰不能にならない
- グローバル低レベルマウスフックを使わない
- `unsafe` は Windows platform binding に限定される

### Phase 2: Usable launcher — strongly desired

実利用に必要な設定保存と操作を追加する。

範囲:

- TOML 設定の読み書き
- アイテム一覧の編集
- ファイルダイアログによるファイル／フォルダ追加
- URL 追加
- 削除、リネーム、並び替え
- Bottom / Top / Left / Right を選ぶ Dock edge 設定 UI
- タイミングと表示設定の永続化
- TOML 不正時の検証と安全なフォールバック
- 基本 fallback アイコン
- edge 変更時の可視 Dock 再配置

Phase 2 の受け入れ条件:

- 設定が再起動後も復元される
- 選択された Dock edge が保存され、ホットゾーン検知に利用される
- 不正設定がアプリをクラッシュさせず、不正ファイルも破壊しない
- アイテムを追加、リネーム、並び替え、削除できる
- 四辺すべてを選べる
- Top / Bottom で横 Dock が動く
- Left / Right で縦 Dock が動く
- UI から URL とローカルパス項目を作成できる

### Phase 3: Windows integration — desired

Windows との統合を強化する。

範囲:

- システムトレイ
- マルチモニター判定と配置
- モニターごとの DPI 調査と補正
- 外部ファイル／フォルダ DnD
- Windows Shell アイコン抽出とキャッシュ
- 起動失敗時の UI 改善
- ログイン時起動の設計調査。実装は必須でない

Phase 3 の受け入れ条件:

- トレイメニューから Dock 表示、設定、正常終了ができる
- カーソルがあるモニターの設定 edge から Dock が表示される
- Dock は表示中に別モニターへ勝手に移動しない
- 負の仮想スクリーン座標でクラッシュしない
- 外部ファイル／フォルダのドロップで永続的な Dock 項目を追加できる
- よく使う `.exe`、`.lnk`、フォルダ、URL に適切なアイコンまたは安定した fallback がある
- アイコンの繰り返し読み込みで明らかな Windows icon/GDI リソースリークがない

### Phase 4: Polish — optional

以下は Phase 1〜3 が安定してから扱う。

- 滑らかな hover magnification
- フォルダスタック／フォルダ展開
- テーマ統合
- Dock の表示／非表示ショートカット
- 壊れた target の検出と修復
- スタートアップ登録
- 起動済みアプリを新規起動せず前面化
- プロファイル
- 高度なコンテキストメニュー

---

## 12. Testing Requirements

### 12.1 Unit tests

少なくとも以下をユニットテストする。

- `DockEdge` の orientation
- 四辺すべてのホットゾーン
- ホットゾーン境界値
- 負のモニター座標
- 四辺すべての Dock anchor 計算
- 表示状態遷移
- reveal hold 時間
- hide delay 時間
- 設定のシリアライズ／デシリアライズ
- 設定検証
- 不正 target の処理
- target 重複検出
- アイテム順序操作
- hover magnification 関数

例:

```rust
#[test]
fn bottom_hot_zone_includes_last_two_rows() {
    let monitor = MonitorRect {
        left: 0,
        top: 0,
        width: 1920,
        height: 1080,
    };

    assert!(is_in_hot_zone(
        DockEdge::Bottom,
        ScreenPoint { x: 960, y: 1078 },
        monitor,
        2,
    ));

    assert!(is_in_hot_zone(
        DockEdge::Bottom,
        ScreenPoint { x: 960, y: 1079 },
        monitor,
        2,
    ));

    assert!(!is_in_hot_zone(
        DockEdge::Bottom,
        ScreenPoint { x: 960, y: 1077 },
        monitor,
        2,
    ));
}
```

### 12.2 Manual validation

以下を手動検証する。

- 四辺すべての表示位置
- Dock の hide/reveal 繰り返し
- `.exe`、`.lnk`、フォルダ、関連付け済みファイル、URL の起動
- 100% と 150% DPI
- 可能なら倍率の異なる 2 台以上のモニター
- 主モニターの左側または上側にあるサブモニター
- タスクバーが下側の場合
- 可能ならタスクバーが別辺の場合
- フォーカス挙動
- スリープ／復帰
- ロック／解除
- フルスクリーンアプリとの相互作用
- トレイメニュー
- 設定ファイル破損からの復帰
- アイコン再読込と Dock 項目編集の繰り返し

手動テスト結果と未解決の制約は `docs/notes.md` に残す。

---

## 13. Error Handling and Logging

`tracing` を使う。

ログレベルの目安:

- `info`: 起動、正常な設定読み込み、Dock 表示／非表示、ユーザー起動操作
- `debug`: 診断用のカーソル座標、モニター矩形、DPI、レイアウト計算
- `warn`: 不正設定からのフォールバック、壊れた Dock target、アイコン fallback
- `error`: Windows API 失敗、設定保存失敗、Shell 起動失敗

要件:

- 存在しないファイル、不正 URL、壊れたショートカット、アイコン抽出失敗、設定解析失敗でクラッシュしない
- 起動失敗時には短く行動可能なメッセージを出す
- Windows API の内部エラーだけをユーザー向けメッセージにしない
- 詳細な原因はログへ残す
- 機密性のあるファイル内容や無関係な環境情報をログに出さない

---

## 14. Documentation Deliverables

### `README.md`

以下を含める。

- プロジェクト目的
- 実装状況
- 前提条件
- ビルド方法
- 実行方法
- 対応する起動対象
- 設定ファイルの保存先
- 既知の制約
- 必要に応じてログを有効化する方法

### `docs/notes.md`

以下を記録する。

- eframe/winit のバージョンと Windows ウィンドウ挙動の確認結果
- hide/reveal を繰り返した結果
- DPI 座標変換の決定
- 利用した Win32 API と採用理由
- 追加した依存 crate、理由、検討した代替案
- トレイ統合の注意点
- アイコン抽出方式とリソース所有権
- 既知の制約と後回しにした作業
- 手動テスト結果

### `docs/acceptance.md`

Phase 1、Phase 2、Phase 3 の受け入れ条件をチェックリスト化する。

---

## 15. Codex Working Rules

実装中は以下を守る。

1. 実装開始前にリポジトリ構成を確認する
2. 指定されていない大きな機能を勝手に追加しない
3. 見た目よりも hide/reveal の信頼性を優先する
4. ドメイン状態、UI 状態、Windows API 呼び出しを分離する
5. `unsafe` を狭く限定し、理由をコメントまたはドキュメントで説明する
6. Windows API の失敗を無視・握りつぶししない
7. 依存 crate を追加する前に、理由と代替案を記録する
8. 必要以上に複雑なアーキテクチャを作らない。ただし本ドキュメントの層境界は守る
9. 意味のあるフェーズごとに `cargo fmt --check`、`cargo clippy -- -D warnings`、`cargo test` を実行する
10. 実行できないコマンドがあれば、理由と未検証部分を明記する
11. API 制約で機能が止まる場合、黙って大きなフレームワークや別技術へ切り替えない
12. 未完了の機能は README と受け入れ文書に明記する
13. 画面端検知のためだけに `WH_MOUSE_LL`、`SetWindowsHookExW`、グローバル入力キャプチャを導入しない
14. 機能が部分実装の場合、対応済み範囲と制約を明確に書く
15. ビルドの再現性を保ち、実機依存の絶対パスはテスト用初期値を除き埋め込まない

---

## 16. First Implementation Task

Phase 1 を最初に実装する。

実装前に行うこと:

1. リポジトリ構成と Rust 開発環境を確認する
2. 最小の Rust プロジェクト構造を作る
3. `docs/notes.md` に初期依存関係の選定理由を短く記録する
4. eframe と接続する前に、純粋な幾何計算と表示状態関数を実装し、テストする
5. Windows のカーソル取得アダプターと Shell 起動アダプターを実装する
6. 表示中の Dock と hide/reveal ループを実装する
7. formatting、lint、test を実行する
8. README、notes、acceptance checklist を実際の検証結果で更新する

Phase 1 完了時に報告すること:

- 変更したファイル一覧
- 追加した依存関係と理由
- 実行したコマンドと結果
- ローカルで検証済みの機能
- 未検証の機能
- 判明した eframe/winit/Win32 の制約
- Phase 2 の推奨実装順
