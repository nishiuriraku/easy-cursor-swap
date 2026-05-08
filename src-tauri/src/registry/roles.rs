//! Windows カーソル役割 (17 種) の定義。
//!
//! `HKCU\Control Panel\Cursors` 配下の値名 (Arrow / Help / ...) と、UI 表示用の
//! 日本語/英語名、`Schemes` 値内でのインデックス順を 1 つの enum に集約する。

use serde::{Deserialize, Serialize};

/// Windows カーソル役割の全17種
/// レジストリ値名をそのまま使用
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CursorRole {
    Arrow,       // 通常の選択
    Help,        // ヘルプの選択
    AppStarting, // バックグラウンドで作業中
    Wait,        // 待ち状態
    Crosshair,   // 領域の選択
    IBeam,       // テキストの選択
    NWPen,       // 手書き
    No,          // 利用不可
    SizeNS,      // 上下に拡大/縮小
    SizeWE,      // 左右に拡大/縮小
    SizeNWSE,    // 斜めに拡大/縮小 1
    SizeNESW,    // 斜めに拡大/縮小 2
    SizeAll,     // 移動
    UpArrow,     // 代替選択
    Hand,        // リンクの選択
    Pin,         // 場所の選択
    Person,      // 人の選択
}

impl CursorRole {
    /// 全17種の役割を返す
    pub fn all() -> &'static [CursorRole] {
        &[
            CursorRole::Arrow,
            CursorRole::Help,
            CursorRole::AppStarting,
            CursorRole::Wait,
            CursorRole::Crosshair,
            CursorRole::IBeam,
            CursorRole::NWPen,
            CursorRole::No,
            CursorRole::SizeNS,
            CursorRole::SizeWE,
            CursorRole::SizeNWSE,
            CursorRole::SizeNESW,
            CursorRole::SizeAll,
            CursorRole::UpArrow,
            CursorRole::Hand,
            CursorRole::Pin,
            CursorRole::Person,
        ]
    }

    /// レジストリ値名を返す
    pub fn registry_name(&self) -> &'static str {
        match self {
            CursorRole::Arrow => "Arrow",
            CursorRole::Help => "Help",
            CursorRole::AppStarting => "AppStarting",
            CursorRole::Wait => "Wait",
            CursorRole::Crosshair => "Crosshair",
            CursorRole::IBeam => "IBeam",
            CursorRole::NWPen => "NWPen",
            CursorRole::No => "No",
            CursorRole::SizeNS => "SizeNS",
            CursorRole::SizeWE => "SizeWE",
            CursorRole::SizeNWSE => "SizeNWSE",
            CursorRole::SizeNESW => "SizeNESW",
            CursorRole::SizeAll => "SizeAll",
            CursorRole::UpArrow => "UpArrow",
            CursorRole::Hand => "Hand",
            CursorRole::Pin => "Pin",
            CursorRole::Person => "Person",
        }
    }

    /// 日本語表示名を返す
    pub fn display_name_ja(&self) -> &'static str {
        match self {
            CursorRole::Arrow => "通常の選択",
            CursorRole::Help => "ヘルプの選択",
            CursorRole::AppStarting => "バックグラウンドで作業中",
            CursorRole::Wait => "待ち状態",
            CursorRole::Crosshair => "領域の選択",
            CursorRole::IBeam => "テキストの選択",
            CursorRole::NWPen => "手書き",
            CursorRole::No => "利用不可",
            CursorRole::SizeNS => "上下に拡大/縮小",
            CursorRole::SizeWE => "左右に拡大/縮小",
            CursorRole::SizeNWSE => "斜めに拡大/縮小 1",
            CursorRole::SizeNESW => "斜めに拡大/縮小 2",
            CursorRole::SizeAll => "移動",
            CursorRole::UpArrow => "代替選択",
            CursorRole::Hand => "リンクの選択",
            CursorRole::Pin => "場所の選択",
            CursorRole::Person => "人の選択",
        }
    }

    /// 英語表示名を返す
    pub fn display_name_en(&self) -> &'static str {
        match self {
            CursorRole::Arrow => "Normal Select",
            CursorRole::Help => "Help Select",
            CursorRole::AppStarting => "Working in Background",
            CursorRole::Wait => "Busy",
            CursorRole::Crosshair => "Precision Select",
            CursorRole::IBeam => "Text Select",
            CursorRole::NWPen => "Handwriting",
            CursorRole::No => "Unavailable",
            CursorRole::SizeNS => "Vertical Resize",
            CursorRole::SizeWE => "Horizontal Resize",
            CursorRole::SizeNWSE => "Diagonal Resize 1",
            CursorRole::SizeNESW => "Diagonal Resize 2",
            CursorRole::SizeAll => "Move",
            CursorRole::UpArrow => "Alternate Select",
            CursorRole::Hand => "Link Select",
            CursorRole::Pin => "Location Select",
            CursorRole::Person => "Person Select",
        }
    }

    /// Schemes 文字列内でのインデックス順序を返す
    pub fn scheme_index(&self) -> usize {
        match self {
            CursorRole::Arrow => 0,
            CursorRole::Help => 1,
            CursorRole::AppStarting => 2,
            CursorRole::Wait => 3,
            CursorRole::Crosshair => 4,
            CursorRole::IBeam => 5,
            CursorRole::NWPen => 6,
            CursorRole::No => 7,
            CursorRole::SizeNS => 8,
            CursorRole::SizeWE => 9,
            CursorRole::SizeNWSE => 10,
            CursorRole::SizeNESW => 11,
            CursorRole::SizeAll => 12,
            CursorRole::UpArrow => 13,
            CursorRole::Hand => 14,
            CursorRole::Pin => 15,
            CursorRole::Person => 16,
        }
    }
}
