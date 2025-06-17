use std::collections::HashMap;
use std::path::Path;
use std::ops::Range;
use std::ffi::OsStr;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::{ThemeSet, Style, Color};
use syntect::easy::HighlightLines;
use regex::Regex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileType {
    Plain,
    Python,
    Rust,
    JavaScript,
    TypeScript,
    HTML,
    CSS,
    Dockerfile,
    YAML,
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
        let syntax_set = SyntaxSet::load_defaults_newlines();
        
        Self {
            syntax_set,
            theme_set: ThemeSet::load_defaults(),
            current_theme: "base16-ocean.dark".to_string(),
            highlight_cache: HashMap::new(),
        }
    }
    
    /// Ensure YAML support is available by adding a basic YAML syntax if needed
    fn ensure_yaml_support(syntax_set: &mut SyntaxSet) {
        // Check if YAML is already available
        if syntax_set.find_syntax_by_extension("yaml").is_some() ||
           syntax_set.find_syntax_by_extension("yml").is_some() {
            return; // YAML already supported
        }
        
        // If not available, we'll create a basic syntax definition
        // For now, we'll rely on fallbacks in the highlight_line method
    }

    pub fn detect_file_type(filename: &str) -> FileType {
        // Special case files
        if filename == "Dockerfile" {
            return FileType::Dockerfile;
        }
        
        // Check for common YAML files without extensions
        match filename.to_lowercase().as_str() {
            ".travis.yml" | ".github/workflows" | "docker-compose.yml" | "docker-compose.yaml" |
            ".gitlab-ci.yml" | "appveyor.yml" | "circle.yml" | "wercker.yml" |
            "ansible.yml" | "playbook.yml" | "site.yml" => return FileType::YAML,
            _ => {}
        }
        
        // Check for files that start with common YAML prefixes
        if filename.starts_with(".github/") && (filename.ends_with(".yml") || filename.ends_with(".yaml")) {
            return FileType::YAML;
        }

        let detected_type = match Path::new(filename)
            .extension()
            .and_then(OsStr::to_str)
        {
            Some("py") => FileType::Python,
            Some("rs") => FileType::Rust,
            Some("js") => FileType::JavaScript,
            Some("ts") => FileType::TypeScript,
            Some("tsx") => FileType::TypeScript, // Add support for .tsx files
            Some("html") | Some("htm") => FileType::HTML,
            Some("css") => FileType::CSS,
            // Enhanced YAML detection
            Some("yaml") | Some("yml") => FileType::YAML,
            _ => FileType::Plain,
        };
        
        detected_type
    }

    pub fn highlight_line<'a>(
        &'a mut self,
        line: &'a str,
        file_type: FileType,
        line_number: usize
    ) -> Vec<(Style, &'a str)> {
        // For YAML files, if syntect doesn't have YAML support, use custom highlighting
        if file_type == FileType::YAML {
            if let Some(custom_highlight) = self.custom_yaml_highlight(line) {
                return custom_highlight;
            }
        }
        
        // Create cache key
        let cache_key = (line.to_string(), line_number);
        
        // Get syntax reference based on file type
        let syntax = match file_type {
            FileType::Plain => self.syntax_set.find_syntax_plain_text(),
            FileType::Python => self.syntax_set.find_syntax_by_extension("py").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text()),
            FileType::Rust => self.syntax_set.find_syntax_by_extension("rs").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text()),
            FileType::JavaScript => self.syntax_set.find_syntax_by_extension("js").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text()),
            FileType::TypeScript => {
                self.syntax_set.find_syntax_by_extension("ts")
                    .or_else(|| self.syntax_set.find_syntax_by_name("TypeScript"))
                    .or_else(|| self.syntax_set.find_syntax_by_name("TypeScript (JavaScript)"))
                    .or_else(|| self.syntax_set.find_syntax_by_extension("js"))
                    .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
            },
            FileType::HTML => self.syntax_set.find_syntax_by_extension("html").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text()),
            FileType::CSS => self.syntax_set.find_syntax_by_extension("css").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text()),
            FileType::Dockerfile => self.syntax_set.find_syntax_by_extension("Dockerfile").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text()),
            FileType::YAML => {
                // First try to find actual YAML syntax
                self.syntax_set.find_syntax_by_extension("yaml")
                    .or_else(|| self.syntax_set.find_syntax_by_extension("yml"))
                    .or_else(|| self.syntax_set.find_syntax_by_name("YAML"))
                    .or_else(|| self.syntax_set.find_syntax_by_name("Yaml"))
                    .or_else(|| self.syntax_set.find_syntax_by_name("yaml"))
                    // If no YAML syntax, use JSON as it's similar structure
                    .or_else(|| self.syntax_set.find_syntax_by_extension("json"))
                    .or_else(|| self.syntax_set.find_syntax_by_name("JSON"))
                    .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
            },
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

    /// Debug function to list all available syntax definitions
    pub fn list_available_syntaxes(&self) -> Vec<String> {
        self.syntax_set.syntaxes().iter()
            .map(|syntax| format!("{} ({})", syntax.name, syntax.file_extensions.join(", ")))
            .collect()
    }

    /// Check if a specific syntax is available by name or extension
    pub fn has_syntax_for_extension(&self, extension: &str) -> bool {
        self.syntax_set.find_syntax_by_extension(extension).is_some()
    }
    
    /// Debug method to check what syntax is being used for a file type
    pub fn debug_syntax_for_filetype(&self, file_type: FileType) -> String {
        let syntax = match file_type {
            FileType::Plain => self.syntax_set.find_syntax_plain_text(),
            FileType::Python => self.syntax_set.find_syntax_by_extension("py").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text()),
            FileType::Rust => self.syntax_set.find_syntax_by_extension("rs").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text()),
            FileType::JavaScript => self.syntax_set.find_syntax_by_extension("js").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text()),
            FileType::TypeScript => {
                self.syntax_set.find_syntax_by_extension("ts")
                    .or_else(|| self.syntax_set.find_syntax_by_name("TypeScript"))
                    .or_else(|| self.syntax_set.find_syntax_by_name("TypeScript (JavaScript)"))
                    .or_else(|| self.syntax_set.find_syntax_by_extension("js"))
                    .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
            },
            FileType::HTML => self.syntax_set.find_syntax_by_extension("html").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text()),
            FileType::CSS => self.syntax_set.find_syntax_by_extension("css").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text()),
            FileType::Dockerfile => self.syntax_set.find_syntax_by_extension("Dockerfile").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text()),
            FileType::YAML => {
                self.syntax_set.find_syntax_by_extension("yaml")
                    .or_else(|| self.syntax_set.find_syntax_by_extension("yml"))
                    .or_else(|| self.syntax_set.find_syntax_by_name("YAML"))
                    .or_else(|| self.syntax_set.find_syntax_by_name("Yaml"))
                    .or_else(|| self.syntax_set.find_syntax_by_name("yaml"))
                    .or_else(|| self.syntax_set.find_syntax_by_name("YML"))
                    .or_else(|| self.syntax_set.find_syntax_by_name("Yet Another Markup Language"))
                    .or_else(|| self.syntax_set.find_syntax_by_extension("json").or_else(|| self.syntax_set.find_syntax_by_name("JSON")))
                    .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
            },
        };
        
        format!("FileType: {:?} -> Syntax: {}", file_type, syntax.name)
    }
    
    /// Custom YAML highlighting when syntect doesn't have YAML support
    fn custom_yaml_highlight<'a>(&self, line: &'a str) -> Option<Vec<(Style, &'a str)>> {
        // Check if native YAML highlighting is available
        if self.syntax_set.find_syntax_by_extension("yaml").is_some() ||
           self.syntax_set.find_syntax_by_extension("yml").is_some() {
            return None; // Use native highlighting
        }
        
        let mut result = Vec::new();
        
        // Create different styles for different YAML elements
        let comment_style = Style {
            foreground: Color { r: 128, g: 128, b: 128, a: 255 }, // Gray for comments
            ..Style::default()
        };
        
        let key_style = Style {
            foreground: Color { r: 100, g: 149, b: 237, a: 255 }, // Blue for keys
            ..Style::default()
        };
        
        let string_style = Style {
            foreground: Color { r: 152, g: 195, b: 121, a: 255 }, // Green for strings
            ..Style::default()
        };
        
        let number_style = Style {
            foreground: Color { r: 209, g: 154, b: 102, a: 255 }, // Orange for numbers
            ..Style::default()
        };
        
        let special_style = Style {
            foreground: Color { r: 198, g: 120, b: 221, a: 255 }, // Purple for special values
            ..Style::default()
        };
        
        // Simple YAML highlighting
        if line.trim_start().starts_with('#') {
            // Comment line
            result.push((comment_style, line));
        } else if line.contains(':') && !line.trim_start().starts_with('-') {
            // Key-value pair
            if let Some(colon_pos) = line.find(':') {
                result.push((key_style, &line[..colon_pos + 1]));
                if colon_pos + 1 < line.len() {
                    result.push((Style::default(), &line[colon_pos + 1..]));
                }
            } else {
                result.push((Style::default(), line));
            }
        } else if line.trim_start().starts_with('-') {
            // List item
            if let Some(dash_pos) = line.find('-') {
                result.push((Style::default(), &line[..dash_pos]));
                result.push((special_style, "-"));
                if dash_pos + 1 < line.len() {
                    result.push((Style::default(), &line[dash_pos + 1..]));
                }
            } else {
                result.push((Style::default(), line));
            }
        } else {
            // Regular text
            result.push((Style::default(), line));
        }
        
        Some(result)
    }
} 

/// Smart indentation rule for a language
#[derive(Debug)]
pub struct IndentRule {
    /// Patterns that increase indent on next line (e.g., ":" in Python, "{" in Rust)
    pub increase_patterns: Vec<Regex>,
    /// Patterns that decrease current line indent (e.g., "else", "}" )
    pub decrease_patterns: Vec<Regex>,
    /// Patterns that both decrease current line and increase next line
    pub decrease_increase_patterns: Vec<Regex>,
}

impl IndentRule {
    pub fn python() -> Self {
        Self {
            increase_patterns: vec![
                Regex::new(r":\s*(?:#.*)?$").unwrap(),           // def, if, for, while, class, etc.
                Regex::new(r"^\s*@\w+").unwrap(),                // decorators
            ],
            decrease_patterns: vec![
                Regex::new(r"^\s*(elif|else|except|finally|break|continue|pass|return)\b").unwrap(),
            ],
            decrease_increase_patterns: vec![
                Regex::new(r"^\s*(elif|else|except|finally).*:\s*(?:#.*)?$").unwrap(),
            ],
        }
    }
    
    pub fn rust() -> Self {
        Self {
            increase_patterns: vec![
                Regex::new(r"\{\s*(?://.*)?$").unwrap(),         // opening brace
                Regex::new(r"=>\s*(?://.*)?$").unwrap(),         // match arms without braces
            ],
            decrease_patterns: vec![
                Regex::new(r"^\s*\}").unwrap(),                 // closing brace
            ],
            decrease_increase_patterns: vec![
                Regex::new(r"^\s*\}\s*else\s*\{").unwrap(),     // } else {
            ],
        }
    }
    
    pub fn javascript() -> Self {
        Self {
            increase_patterns: vec![
                Regex::new(r"\{\s*(?://.*)?$").unwrap(),         // opening brace
                Regex::new(r"=>\s*(?://.*)?$").unwrap(),         // arrow functions
            ],
            decrease_patterns: vec![
                Regex::new(r"^\s*\}").unwrap(),                 // closing brace
            ],
            decrease_increase_patterns: vec![
                Regex::new(r"^\s*\}\s*else\s*\{").unwrap(),     // } else {
                Regex::new(r"^\s*\}\s*catch\s*\(").unwrap(),    // } catch (
                Regex::new(r"^\s*\}\s*finally\s*\{").unwrap(),  // } finally {
            ],
        }
    }
    
    pub fn html() -> Self {
        Self {
            increase_patterns: vec![
                // Opening tags (simplified pattern without lookahead)
                Regex::new(r"<[a-zA-Z][^/>]*>$").unwrap(),     // Basic opening tags
                Regex::new(r"<(div|p|ul|ol|li|table|tr|td|th|head|body|html|section|article|nav|aside|header|footer|main)[^>]*>").unwrap(), // Common block elements
            ],
            decrease_patterns: vec![
                Regex::new(r"^\s*</").unwrap(),                 // closing tags
            ],
            decrease_increase_patterns: vec![],
        }
    }

    pub fn css() -> Self {
        Self {
            increase_patterns: vec![
                Regex::new(r"\{\s*(?:/\*.*\*/\s*)?$").unwrap(), // opening brace
            ],
            decrease_patterns: vec![
                Regex::new(r"^\s*\}").unwrap(),                 // closing brace
            ],
            decrease_increase_patterns: vec![],
        }
    }

    pub fn yaml() -> Self {
        Self {
            increase_patterns: vec![
                Regex::new(r":\s*$").unwrap(),                   // key: (ending with colon)
                Regex::new(r":\s*\|").unwrap(),                  // literal block scalar |
                Regex::new(r":\s*>").unwrap(),                   // folded block scalar >
                Regex::new(r"^\s*-\s*$").unwrap(),               // list item with no content
                Regex::new(r"^\s*-\s+\w+:\s*$").unwrap(),        // list item with key:
            ],
            decrease_patterns: vec![
                // YAML doesn't typically have decrease patterns like braces
            ],
            decrease_increase_patterns: vec![],
        }
    }
}

/// Smart indentation engine
pub struct SmartIndenter {
    rules: HashMap<FileType, IndentRule>,
}

impl SmartIndenter {
    pub fn new() -> Self {
        let mut rules = HashMap::new();
        rules.insert(FileType::Python, IndentRule::python());
        rules.insert(FileType::Rust, IndentRule::rust());
        rules.insert(FileType::JavaScript, IndentRule::javascript());
        rules.insert(FileType::TypeScript, IndentRule::javascript()); // Same as JS
        rules.insert(FileType::HTML, IndentRule::html());
        rules.insert(FileType::CSS, IndentRule::css());
        rules.insert(FileType::YAML, IndentRule::yaml()); // Add YAML support
        
        Self { rules }
    }
    
    /// Calculate the indent for a new line based on the previous lines
    pub fn calculate_indent(
        &self,
        lines: &[String],
        current_line_idx: usize,
        current_line_content: &str,
        file_type: FileType,
        tab_size: usize,
    ) -> usize {
        let rule = match self.rules.get(&file_type) {
            Some(rule) => rule,
            None => return self.get_previous_indent(lines, current_line_idx, tab_size), // fallback
        };
        
        if lines.is_empty() {
            return 0;
        }
        
        // If we're calculating for a new line beyond the current lines,
        // use the last line as the "previous" line
        let prev_line_idx = if current_line_idx >= lines.len() {
            lines.len() - 1
        } else if current_line_idx == 0 {
            return 0;
        } else {
            current_line_idx - 1
        };
        
        let prev_line = &lines[prev_line_idx];
        let prev_indent = self.get_line_indent(prev_line, tab_size);
        
        // Check if current line should decrease indent
        if rule.decrease_patterns.iter().any(|pattern| pattern.is_match(current_line_content)) {
            return prev_indent.saturating_sub(tab_size);
        }
        
        // Check if current line should both decrease and increase
        if rule.decrease_increase_patterns.iter().any(|pattern| pattern.is_match(current_line_content)) {
            return prev_indent; // Same as previous
        }
        
        // Special case for Python: if __name__ == '__main__' at top level should stay at top level
        if file_type == FileType::Python && prev_indent == 0 && prev_line.trim().contains("__name__") && prev_line.trim().contains("__main__") {
            return 0; // Don't indent after main guard at top level
        }
        
        // Check if previous line should increase indent
        if rule.increase_patterns.iter().any(|pattern| pattern.is_match(prev_line)) {
            return prev_indent + tab_size;
        }
        
        prev_indent
    }
    
    pub fn get_line_indent(&self, line: &str, tab_size: usize) -> usize {
        let mut count = 0;
        for ch in line.chars() {
            match ch {
                ' ' => count += 1,
                '\t' => count += tab_size,
                _ => break,
            }
        }
        count
    }
    
    fn get_previous_indent(&self, lines: &[String], current_line_idx: usize, tab_size: usize) -> usize {
        if current_line_idx == 0 {
            return 0;
        }
        
        let prev_line = &lines[current_line_idx - 1];
        self.get_line_indent(prev_line, tab_size)
    }
}

// Extend the existing SyntaxHighlighter with smart indentation
impl SyntaxHighlighter {
    pub fn create_smart_indenter() -> SmartIndenter {
        SmartIndenter::new()
    }
}