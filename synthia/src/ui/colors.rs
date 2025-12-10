/// Pastel color palette for terminal UI
/// These colors are designed to work well on both light and dark backgrounds
use crossterm::style::Color;

/// Pastel colors that work on both light and dark themes
/// Uses RGB values for precise control
pub struct PastelColors;

impl PastelColors {
    /// Soft blue for headers and branding (was Color::Blue)
    /// RGB(135, 206, 250) - Light Sky Blue
    pub const HEADER: Color = Color::Rgb { r: 135, g: 206, b: 250 };

    /// Soft cyan for assistant text (was Color::Cyan)
    /// RGB(152, 245, 225) - Aquamarine
    pub const ASSISTANT: Color = Color::Rgb { r: 152, g: 245, b: 225 };

    /// Soft yellow for tools and warnings (was Color::Yellow)
    /// RGB(255, 229, 180) - Peach
    pub const TOOL: Color = Color::Rgb { r: 255, g: 229, b: 180 };

    /// Soft green for success (was Color::Green)
    /// RGB(182, 255, 182) - Mint Green
    pub const SUCCESS: Color = Color::Rgb { r: 182, g: 255, b: 182 };

    /// Soft red for errors (was Color::Red)
    /// RGB(255, 182, 193) - Light Pink
    pub const ERROR: Color = Color::Rgb { r: 255, g: 182, b: 193 };

    /// Soft lavender for prompts and UI elements
    /// RGB(230, 230, 250) - Lavender
    #[allow(dead_code)]
    pub const PROMPT: Color = Color::Rgb { r: 230, g: 230, b: 250 };
}
