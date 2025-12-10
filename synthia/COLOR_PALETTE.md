# Synthia Color Palette

Synthia uses a carefully selected pastel color palette that works well on both light and dark terminal themes.

## Color Reference

| Usage | Color Name | RGB Value | Description |
|-------|-----------|-----------|-------------|
| Headers & Branding | Light Sky Blue | `RGB(135, 206, 250)` | Soft blue for the app header and decorative borders |
| Assistant Text | Aquamarine | `RGB(152, 245, 225)` | Soft cyan for "Synthia:" prefix and assistant responses |
| Tools & Warnings | Peach | `RGB(255, 229, 180)` | Soft yellow for tool names, system messages, and modal borders |
| Success Messages | Mint Green | `RGB(182, 255, 182)` | Soft green for successful operations (✓ checkmarks) |
| Error Messages | Light Pink | `RGB(255, 182, 193)` | Soft red for errors (✗ marks) |
| Prompts (Reserved) | Lavender | `RGB(230, 230, 250)` | Soft purple for future UI elements |

## Design Principles

1. **Accessibility**: All colors have sufficient contrast on both dark and light backgrounds
2. **Consistency**: Each color has a specific semantic meaning throughout the UI
3. **Professional**: Pastel tones are softer and less jarring than primary colors
4. **Theme-Agnostic**: Works equally well in dark mode (black background) and light mode (white background)

## Before & After

### Old Colors (Primary)
- **Red** (`RGB(255, 0, 0)`) → Too harsh, eye-straining
- **Blue** (`RGB(0, 0, 255)`) → Too intense
- **Yellow** (`RGB(255, 255, 0)`) → Too bright
- **Green** (`RGB(0, 255, 0)`) → Too neon
- **Cyan** (`RGB(0, 255, 255)`) → Too electric

### New Colors (Pastel)
- **Light Pink** (`RGB(255, 182, 193)`) → Gentle, less alarming for errors
- **Light Sky Blue** (`RGB(135, 206, 250)`) → Professional, calming
- **Peach** (`RGB(255, 229, 180)`) → Warm, readable
- **Mint Green** (`RGB(182, 255, 182)`) → Positive, soothing
- **Aquamarine** (`RGB(152, 245, 225)`) → Cool, tech-friendly

## Usage in Code

Colors are defined in `synthia/src/ui/colors.rs`:

```rust
use crate::ui::colors::PastelColors;

// Headers
SetForegroundColor(PastelColors::HEADER)

// Assistant text
SetForegroundColor(PastelColors::ASSISTANT)

// Tool names
SetForegroundColor(PastelColors::TOOL)

// Success
SetForegroundColor(PastelColors::SUCCESS)

// Errors
SetForegroundColor(PastelColors::ERROR)
```

## Testing

To see the color palette in action, run synthia and observe:

1. **Header**: Light sky blue border at startup
2. **Assistant**: Aquamarine "Synthia:" prefix
3. **Tools**: Peach tool names when executing
4. **Success**: Mint green ✓ when tools complete successfully
5. **Errors**: Light pink ✗ when tools fail

The colors should look pleasant and readable regardless of your terminal theme (light or dark).
