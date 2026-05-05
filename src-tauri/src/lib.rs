//! CursorForge - Rust バックエンドのモジュール構成
//! 
//! 各モジュールの役割:
//! - config: アプリケーション設定の読み書き（Source of Truth）
//! - registry: Windows レジストリ操作（カーソル適用・復旧）
//! - cursor: .cur バイナリ生成、画像処理パイプライン
//! - theme: テーマパッケージ (.cursorpack) の管理
//! - tray: システムトレイ常駐ロジック
//! - darkmode: ダークモード監視と自動切替
//! - commands: Tauri IPC コマンド定義
//! - errors: エラー型定義

pub mod backup;
pub mod commands;
pub mod config;
pub mod cursor;
pub mod cursor_watcher;
pub mod darkmode;
pub mod errors;
pub mod keystore;
pub mod logging;
pub mod marketplace;
pub mod registry;
pub mod single_instance;
pub mod theme;
pub mod tray;
