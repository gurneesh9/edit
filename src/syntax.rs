use std::collections::HashMap;
use std::path::Path;
use std::ops::Range;
use std::ffi::OsStr;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::{ThemeSet, Style};
use syntect::easy::HighlightLines;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Plain,
    Python,
    Rust,
    JavaScript,
    TypeScript,
    HTML,
    CSS,
    // Add more as needed
}

pub struct HighlightedText<'a> {
    pub text: &'a str,
    pub styles: Vec<(Style, Range<usize>)>,
}

pub struct SyntaxHighlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    current_theme: String,
    highlight_cache: HashMap<(String, usize), Vec<(Style, String)>>,
}

impl SyntaxHighlighter {
    const MAX_CACHE_SIZE: usize = 1000;

    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            current_theme: "base16-ocean.dark".to_string(),
            highlight_cache: HashMap::new(),
        }
    }

    pub fn detect_file_type(filename: &str) -> FileType {
        match Path::new(filename)
            .extension()
            .and_then(OsStr::to_str)
        {
            Some("py") => FileType::Python,
            Some("rs") => FileType::Rust,
            Some("js") => FileType::JavaScript,
            Some("ts") => FileType::TypeScript,
            Some("html") | Some("htm") => FileType::HTML,
            Some("css") => FileType::CSS,
            _ => FileType::Plain,
        }
    }

    pub fn highlight_line<'a>(
        &'a mut self,
        line: &'a str,
        file_type: FileType,
        line_number: usize
    ) -> Vec<(Style, &'a str)> {
        // Create cache key
        let cache_key = (line.to_string(), line_number);
        
        // Get syntax reference based on file type
        let syntax = match file_type {
            FileType::Plain => self.syntax_set.find_syntax_plain_text(),
            FileType::Python => self.syntax_set.find_syntax_by_extension("py").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text()),
            FileType::Rust => self.syntax_set.find_syntax_by_extension("rs").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text()),
            FileType::JavaScript => self.syntax_set.find_syntax_by_extension("js").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text()),
            FileType::TypeScript => self.syntax_set.find_syntax_by_extension("ts").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text()),
            FileType::HTML => self.syntax_set.find_syntax_by_extension("html").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text()),
            FileType::CSS => self.syntax_set.find_syntax_by_extension("css").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text()),
        };

        // Perform highlighting
        let mut highlighter = HighlightLines::new(
            syntax,
            &self.theme_set.themes[&self.current_theme]
        );

        let highlighted = highlighter.highlight_line(line, &self.syntax_set)
            .unwrap_or_else(|_| vec![(Style::default(), line)]);

        // Cache the result for future use
        let cached: Vec<(Style, String)> = highlighted.iter()
            .map(|(style, text)| (*style, text.to_string()))
            .collect();
        
        if self.highlight_cache.len() >= Self::MAX_CACHE_SIZE {
            self.prune_cache_if_needed();
        }
        self.highlight_cache.insert(cache_key, cached);

        // Return the original highlighted result
        highlighted
    }

    fn prune_cache_if_needed(&mut self) {
        if self.highlight_cache.len() > Self::MAX_CACHE_SIZE {
            let to_remove = self.highlight_cache.len() - Self::MAX_CACHE_SIZE;
            let keys: Vec<_> = self.highlight_cache.keys()
                .take(to_remove)
                .cloned()
                .collect();
            for key in keys {
                self.highlight_cache.remove(&key);
            }
        }
    }

    pub fn clear_cache(&mut self) {
        self.highlight_cache.clear();
    }

    pub fn set_theme(&mut self, theme_name: &str) -> bool {
        if self.theme_set.themes.contains_key(theme_name) {
            self.current_theme = theme_name.to_string();
            self.clear_cache();
            true
        } else {
            false
        }
    }

    pub fn load_custom_theme(&mut self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let theme = ThemeSet::get_theme(path)?;
        let theme_name = path.file_stem()
            .and_then(OsStr::to_str)
            .unwrap_or("custom")
            .to_string();
        
        self.theme_set.themes.insert(theme_name.clone(), theme);
        self.current_theme = theme_name;
        self.clear_cache();
        Ok(())
    }

    pub fn available_themes(&self) -> Vec<String> {
        self.theme_set.themes.keys()
            .map(|k| k.to_string())
            .collect()
    }
} 