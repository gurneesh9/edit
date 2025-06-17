// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use edit::framebuffer::IndexedColor;
use edit::input::{kbmod, vk};
use edit::tui::*;
use edit::syntax::FileType;

use crate::documents::Document;
use crate::state::*;

/// Studio Ghibli themed tab bar with magical touches
pub fn draw_ghibli_tab_bar(ctx: &mut Context, state: &mut State) {
    // Check if we should show tabs first
    let document_count = state.documents.len();
    if document_count <= 1 {
        return;
    }

    // Handle shortcuts first (before borrowing documents)
    if let Some(key) = ctx.keyboard_input() {
        // Alt+Left/Right: Navigate between tabs (like browser tabs)
        if key == kbmod::ALT | vk::RIGHT {
            state.documents.switch_to_next();
            ctx.needs_rerender();
            return;
        } else if key == kbmod::ALT | vk::LEFT {
            state.documents.switch_to_previous();
            ctx.needs_rerender();
            return;
        }
        // Ctrl+PageUp/PageDown: Alternative tab navigation
        else if key == kbmod::CTRL | vk::NEXT {  // PageDown
            state.documents.switch_to_next();
            ctx.needs_rerender();
            return;
        } else if key == kbmod::CTRL | vk::PRIOR { // PageUp
            state.documents.switch_to_previous();
            ctx.needs_rerender();
            return;
        }
        // F1-F9: Switch to specific tab (F keys are usually free)
        else {
            for i in 1..=9 {
                let vk_fkey = match i {
                    1 => vk::F1, 2 => vk::F2, 3 => vk::F3, 4 => vk::F4, 5 => vk::F5,
                    6 => vk::F6, 7 => vk::F7, 8 => vk::F8, 9 => vk::F9,
                    _ => continue,
                };
                
                if key == vk_fkey {
                    if state.documents.switch_to_index(i - 1) {
                        ctx.needs_rerender();
                    }
                    return;
                }
            }
        }
    }

    // Get documents after handling shortcuts
    let documents = state.documents.all_documents();
    let active_index = state.documents.active_index().unwrap_or(0);

    ctx.block_begin("ghibli_tab_bar");
    // Warm background like tree bark - using available colors
    ctx.attr_background_rgba(ctx.indexed(IndexedColor::Black));
    ctx.attr_foreground_rgba(ctx.indexed(IndexedColor::Yellow)); // Warm text
    
    // Create a single label with all tab information
    let mut tab_display = String::new();
    for (index, doc) in documents.iter().enumerate() {
        let is_active = index == active_index;
        let is_dirty = doc.buffer.borrow().is_dirty();
        
        // Magical tab content with emoji based on file type
        let file_icon = match doc.file_type {
            FileType::Rust => "🦀",
            FileType::JavaScript => "⚡",
            FileType::TypeScript => "💙", 
            FileType::Python => "🐍",
            FileType::HTML => "🌐",
            FileType::CSS => "🎨",
            FileType::YAML => "⚙️",
            _ => "📄",
        };
        
        let display_name = get_display_name(doc);
        let tab_text = if is_dirty {
            format!("{} ● {}", file_icon, display_name)
        } else {
            format!("{} {}", file_icon, display_name)
        };
        
        // Mark active tab
        if is_active {
            tab_display.push_str(&format!("[{}]", tab_text));
        } else {
            tab_display.push_str(&format!(" {} ", tab_text));
        }
        
        // Add separator
        if index < documents.len() - 1 {
            tab_display.push_str(" 🌿 ");
        }
    }
    
    // Display the tabs as a single label
    ctx.label("tabs_display", &tab_display);
    
    // Add navigation hint with the correct shortcuts
    ctx.label("tab_hint", " [Alt+← →: Navigate | F1-F9: Jump | Ctrl+PgUp/PgDn: Switch]");
    ctx.attr_foreground_rgba(ctx.indexed(IndexedColor::BrightBlack)); // Dimmed text
    
    ctx.block_end();
}

fn get_display_name(doc: &Document) -> String {
    if doc.filename.is_empty() {
        "Untitled".to_string()
    } else {
        // Show just the filename, not the full path
        doc.filename.clone()
    }
}
