//! HTML 解析与 Markdown 转换
//!
//! 负责将微信文章 HTML 解析为结构化数据。
//! 支持标题/作者/时间提取、HTML→Markdown 转换、表格/公式/图片提取。

use regex::Regex;
use scraper::{ElementRef, Html, Node, Selector};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// 解析后的文章数据
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArticleData {
    pub title: String,
    pub author: String,
    pub publish_time: String,
    /// 纯文本正文（向后兼容）
    pub content: String,
    /// Markdown 格式正文（保留标题/列表/引用/图片等结构）
    pub content_markdown: String,
    /// 正文中的图片 URL 列表
    pub images: Vec<String>,
}

// ── CSS 选择器（主 + 备用，适应不同页面版本）──

/// 标题选择器：微信文章标题通常使用 `h1#activity-name` 或 `h1.rich_media_title`
fn title_selectors() -> &'static Vec<Selector> {
    static SEL: OnceLock<Vec<Selector>> = OnceLock::new();
    SEL.get_or_init(|| {
        vec![
            Selector::parse("h1#activity-name").unwrap(),
            Selector::parse("h1.rich_media_title").unwrap(),
        ]
    })
}

/// 作者选择器：可能使用 span、a 标签，3 个选择器兜底
fn author_selectors() -> &'static Vec<Selector> {
    static SEL: OnceLock<Vec<Selector>> = OnceLock::new();
    SEL.get_or_init(|| {
        vec![
            Selector::parse("span#js_author_name").unwrap(),
            Selector::parse("span.rich_media_meta_nickname").unwrap(),
            Selector::parse("a#js_name").unwrap(),
        ]
    })
}

/// 发布时间选择器：可能使用 em 标签
fn publish_time_selectors() -> &'static Vec<Selector> {
    static SEL: OnceLock<Vec<Selector>> = OnceLock::new();
    SEL.get_or_init(|| {
        vec![
            Selector::parse("em#publish_time").unwrap(),
            Selector::parse("em.rich_media_meta_text").unwrap(),
        ]
    })
}

/// 正文内容选择器：微信文章正文在 `div#js_content` 或 `div.rich_media_content` 中
fn content_selectors() -> &'static Vec<Selector> {
    static SEL: OnceLock<Vec<Selector>> = OnceLock::new();
    SEL.get_or_init(|| {
        vec![
            Selector::parse("div#js_content").unwrap(),
            Selector::parse("div.rich_media_content").unwrap(),
        ]
    })
}

// ── 预编译正则表达式 ──

/// 匹配连续 3 个以上的换行符，用于合并多余空行
fn newlines_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\n{3,}").unwrap())
}

/// 匹配连续 2 个以上的空格，用于压缩多余空格
fn spaces_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r" {2,}").unwrap())
}

/// 微信文章 HTML 解析器
///
/// 将 HTML 转换为结构化数据，支持：
/// - 多选择器兜底提取标题/作者/时间
/// - HTML → Markdown 格式转换（保留标题、列表、引用、图片、表格、公式）
/// - 图片 URL 自动提取
pub struct WeixinParser;

impl WeixinParser {
    /// 创建解析器实例
    pub fn new() -> Self {
        Self
    }

    /// 解析微信文章 HTML，提取全部结构化数据
    ///
    /// 执行流程：
    /// 1. 提取标题（标准选择器 → meta og:title → meta twitter:title）
    /// 2. 提取作者、发布时间
    /// 3. 提取正文纯文本和 Markdown
    /// 4. 提取图片 URL 列表
    /// 5. 标题反补：如果正文第一个 H1 比提取的标题更长，取正文标题
    /// 6. 发布时间兜底：从 meta 标签取
    pub fn parse(&self, html: &str) -> ArticleData {
        let document = Html::parse_document(html);

        let author = self.extract_text(&document, author_selectors(), "未知作者");
        let publish_time = self.extract_text(&document, publish_time_selectors(), "");
        let images = self.extract_images(&document);
        let content = self.extract_content_plain(&document);
        let content_markdown = self.content_to_markdown(&document);

        // 标题提取：多优先级兜底
        // 1) og:title meta 标签（最可靠，微信文章发布时必填）
        // 2) twitter:title meta 标签
        // 3) 标准 CSS 选择器
        // 4) 正文 Markdown 的第一个 # 标题（仅当其他方式都失败时）
        let title = {
            if let Some(mt) = self.extract_meta_content(&document, "og:title") {
                mt
            } else if let Some(mt) = self.extract_meta_content(&document, "twitter:title") {
                mt
            } else {
                let t = self.extract_text(&document, title_selectors(), "");
                if !t.is_empty() {
                    t
                } else if let Some(h) = self.extract_first_heading(&content_markdown) {
                    h
                } else {
                    String::new()
                }
            }
        };

        let title = if title.is_empty() {
            "未找到标题".to_string()
        } else {
            title
        };

        // 将文章标题作为 H1 插入 Markdown 正文开头
        let content_markdown = if !title.is_empty() && title != "未找到标题" {
            format!("# {}\n\n{}", title, content_markdown)
        } else {
            content_markdown
        };

        // 发布时间兜底：尝试 meta 标签
        let publish_time = if !publish_time.is_empty() {
            publish_time
        } else if let Some(t) = self.extract_meta_content(&document, "article:published_time")
            .or_else(|| self.extract_meta_content(&document, "og:updated_time"))
        {
            t
        } else {
            "未知时间".to_string()
        };

        ArticleData {
            title,
            author,
            publish_time,
            content,
            content_markdown,
            images,
        }
    }

    // ── 通用提取方法 ──

    /// 通用文本提取：按选择器列表逐个尝试，取第一个非空结果
    fn extract_text(&self, doc: &Html, selectors: &[Selector], fallback: &str) -> String {
        for sel in selectors {
            if let Some(el) = doc.select(sel).next() {
                let text = el.text().collect::<Vec<_>>().join("").trim().to_string();
                if !text.is_empty() {
                    return text;
                }
            }
        }
        fallback.to_string()
    }

    /// 从 `<meta property="xxx">` 标签提取 `content` 属性值
    fn extract_meta_content(&self, doc: &Html, property: &str) -> Option<String> {
        let selector_str = format!("meta[property='{}']", property);
        let sel = Selector::parse(&selector_str).ok()?;
        let el = doc.select(&sel).next()?;
        elem_attr(el.value(), "content")
    }

    /// 从 Markdown 内容中提取第一个 # 标题（跳过代码块内的假标题）
    fn extract_first_heading(&self, markdown: &str) -> Option<String> {
        let mut in_code_block = false;
        for line in markdown.lines() {
            if line.starts_with("```") {
                in_code_block = !in_code_block;
                continue;
            }
            if !in_code_block && line.starts_with("# ") {
                let heading = line.trim_start_matches("# ").trim().to_string();
                if !heading.is_empty() {
                    return Some(heading);
                }
            }
        }
        None
    }

    /// 提取正文纯文本（向后兼容）
    ///
    /// 从正文容器中提取纯文本，移除所有 HTML 标签。
    fn extract_content_plain(&self, doc: &Html) -> String {
        for sel in content_selectors() {
            if let Some(content_el) = doc.select(sel).next() {
                let inner_html = content_el.inner_html();
                let fragment = Html::parse_fragment(&inner_html);
                let text: String = fragment
                    .root_element()
                    .text()
                    .collect::<Vec<_>>()
                    .join("\n");
                let cleaned = self.clean_text(&text);
                if !cleaned.is_empty() {
                    return cleaned;
                }
            }
        }
        "未找到正文内容".to_string()
    }

    /// 提取正文中所有图片的真实 URL
    ///
    /// 微信使用懒加载：真实 URL 在 `data-src`，`src` 可能是 base64 占位图。
    fn extract_images(&self, doc: &Html) -> Vec<String> {
        let img_selector = Selector::parse("img").unwrap();
        let mut images = Vec::new();
        for el in doc.select(&img_selector) {
            let src = elem_attr(el.value(), "data-src")
                .or_else(|| elem_attr(el.value(), "src"))
                .filter(|s| !s.starts_with("data:"));
            if let Some(url) = src {
                images.push(url);
            }
        }
        images
    }

    // ── Markdown 转换 ──

    /// 将正文 HTML 转换为 Markdown
    ///
    /// 从正文容器中提取 HTML，递归遍历 DOM 树转换为 Markdown。
    fn content_to_markdown(&self, doc: &Html) -> String {
        for sel in content_selectors() {
            if let Some(content_el) = doc.select(sel).next() {
                let inner_html = content_el.inner_html();
                let fragment = Html::parse_fragment(&inner_html);
                let result = self.element_to_markdown(fragment.root_element());
                let cleaned = self.clean_text(&result);
                if !cleaned.is_empty() {
                    return cleaned;
                }
            }
        }
        String::new()
    }

    /// 递归转换 ElementRef 及其子节点为 Markdown 字符串
    ///
    /// 遍历所有子节点，文本节点直接拼接，元素节点调用 `push_element_markdown` 处理。
    fn element_to_markdown(&self, elem: ElementRef) -> String {
        let mut parts = Vec::new();
        for child in elem.children() {
            match child.value() {
                Node::Text(t) => {
                    let text = t.text.trim();
                    if !text.is_empty() {
                        parts.push(text.to_string());
                    }
                }
                Node::Element(_) => {
                    if let Some(child_elem) = ElementRef::wrap(child) {
                        self.push_element_markdown(child_elem, &mut parts);
                    }
                }
                _ => {}
            }
        }
        parts.concat()
    }

    /// 将单个 Element 按标签类型转换为 Markdown，追加到 parts
    ///
    /// 支持的标签：
    /// - 块级：p/section/div → 段落 + 换行
    /// - 标题：h1~h6 → #~######
    /// - 图片：img → ![](url)
    /// - 样式：strong/b/em/i/code → **bold**/*italic*/`code`
    /// - 链接：a → [text](url)
    /// - 代码：pre → ```code```
    /// - 引用：blockquote → > text
    /// - 列表：ul/ol → - / 1. 列表项
    /// - 表格：table → Markdown 表格
    /// - 公式：span.math → $formula$ / $$formula$$
    /// - 其他：br/h/透传子元素
    fn push_element_markdown(&self, elem: ElementRef, parts: &mut Vec<String>) {
        let el = elem.value();
        let tag = &*el.name.local;
        match tag {
            // 块级元素：段落，末尾加双换行
            "p" | "section" | "div" => {
                let inner = self.element_to_markdown(elem);
                let trimmed = inner.trim();
                if !trimmed.is_empty() {
                    // 纯粗体段落视为 H3 子标题
                    if trimmed.starts_with("**") && trimmed.ends_with("**")
                        && trimmed.matches("**").count() == 2
                    {
                        let heading = &trimmed[2..trimmed.len() - 2];
                        parts.push(format!("### {}", heading));
                        parts.push("\n\n".to_string());
                    } else {
                        parts.push(trimmed.to_string());
                        parts.push("\n\n".to_string());
                    }
                }
            }

            // 标题：h1 跳过（保留给文章标题），h2~h6 正常映射
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                let level = tag[1..].parse::<usize>().unwrap_or(1);
                let inner = self.element_to_markdown(elem);
                let trimmed = inner.trim();
                if !trimmed.is_empty() {
                    if level == 1 {
                        // H1 跳过，不重复出现在正文中
                        parts.push(trimmed.to_string());
                        parts.push("\n\n".to_string());
                    } else {
                        parts.push(format!("{} {}", "#".repeat(level), trimmed));
                        parts.push("\n\n".to_string());
                    }
                }
            }

            // 图片：优先取 data-src（微信懒加载），降级到 src
            "img" => {
                let src = elem_attr(el, "data-src")
                    .or_else(|| elem_attr(el, "src"))
                    .filter(|s| !s.starts_with("data:"));
                if let Some(url) = src {
                    let alt = elem_attr(el, "alt").unwrap_or_default();
                    parts.push(format!("![{}]({})", alt, url));
                    parts.push("\n\n".to_string());
                }
            }

            // 换行
            "br" => {
                parts.push("\n".to_string());
            }

            // 粗体
            "strong" | "b" => {
                let inner = self.element_to_markdown(elem);
                let trimmed = inner.trim();
                if !trimmed.is_empty() {
                    parts.push(format!("**{}**", trimmed));
                }
            }

            // 斜体
            "em" | "i" => {
                let inner = self.element_to_markdown(elem);
                let trimmed = inner.trim();
                if !trimmed.is_empty() {
                    parts.push(format!("*{}*", trimmed));
                }
            }

            // 链接
            "a" => {
                let href = elem_attr(el, "href").unwrap_or_default();
                let inner = self.element_to_markdown(elem);
                let trimmed = inner.trim().to_string();
                if !trimmed.is_empty() && !href.is_empty() {
                    parts.push(format!("[{}]({})", trimmed, href));
                } else if !trimmed.is_empty() {
                    parts.push(trimmed);
                }
            }

            // 行内代码
            "code" => {
                let inner = self.element_to_markdown(elem);
                let trimmed = inner.trim();
                if !trimmed.is_empty() {
                    parts.push(format!("`{}`", trimmed));
                }
            }

            // 代码块：直接提取文本，避免 inner <code> 再套一层反引号
            "pre" => {
                let text: String = elem.text().collect::<Vec<_>>().join("");
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    parts.push(format!("\n```\n{}\n```\n", trimmed));
                }
            }

            // 引用
            "blockquote" => {
                let inner = self.element_to_markdown(elem);
                let trimmed = inner.trim();
                if !trimmed.is_empty() {
                    let quoted: Vec<String> =
                        trimmed.lines().map(|l| format!("> {}", l)).collect();
                    parts.push(quoted.join("\n"));
                    parts.push("\n\n".to_string());
                }
            }

            // 列表
            "ul" | "ol" => {
                self.process_list(elem, tag == "ol", parts);
            }

            // 表格
            "table" => {
                self.process_table(elem, parts);
            }

            // 数学公式（MathJax / KaTeX 渲染）
            "span" => {
                let class_attr = elem_attr(el, "class").unwrap_or_default();
                if class_attr.contains("math")
                    || class_attr.contains("MathJax")
                    || class_attr.contains("katex")
                    || class_attr.contains("formula")
                {
                    let inner = self.element_to_markdown(elem);
                    let trimmed = inner.trim();
                    if !trimmed.is_empty() {
                        let is_inline = class_attr.contains("inline") || tag == "span";
                        if is_inline {
                            parts.push(format!("${}$", trimmed));
                        } else {
                            parts.push(format!("\n$$\n{}\n$$\n\n", trimmed));
                        }
                    }
                } else {
                    // 普通 span，透传子元素
                    let inner = self.element_to_markdown(elem);
                    let trimmed = inner.trim();
                    if !trimmed.is_empty() {
                        parts.push(trimmed.to_string());
                    }
                }
            }

            // 表格行/单元格：由 process_table 统一处理，递归时跳过加粗等样式
            "tr" | "th" | "td" | "thead" | "tbody" | "tfoot" | "caption" | "colgroup"
            | "col" => {
                let inner = self.element_to_markdown(elem);
                let trimmed = inner.trim();
                if !trimmed.is_empty() {
                    parts.push(trimmed.to_string());
                }
            }

            // 水平线
            "hr" => {
                parts.push("\n---\n\n".to_string());
            }

            // 其他标签：透传子元素
            _ => {
                let inner = self.element_to_markdown(elem);
                let trimmed = inner.trim();
                if !trimmed.is_empty() {
                    parts.push(trimmed.to_string());
                }
            }
        }
    }

    /// 处理列表（ul/ol）
    ///
    /// 将 `<li>` 转换为 `- `（无序）或 `1. `（有序）格式。
    fn process_list(&self, elem: ElementRef, ordered: bool, parts: &mut Vec<String>) {
        for (idx, child) in elem.children().enumerate() {
            if let Some(li) = ElementRef::wrap(child) {
                if li.value().name.local.as_ref() == "li" {
                    let inner = self.element_to_markdown(li);
                    let trimmed = inner.trim();
                    if !trimmed.is_empty() {
                        if ordered {
                            parts.push(format!("{}. {}", idx + 1, trimmed));
                        } else {
                            parts.push(format!("- {}", trimmed));
                        }
                        parts.push("\n".to_string());
                    }
                }
            }
        }
        parts.push("\n".to_string());
    }

    /// 处理表格（table → Markdown 表格）
    ///
    /// 收集所有行和单元格，渲染为 Markdown 表格格式：
    /// ```markdown
    /// | 表头1 | 表头2 |
    /// |-------|-------|
    /// | 单元格 | 单元格 |
    /// ```
    fn process_table(&self, elem: ElementRef, parts: &mut Vec<String>) {
        let mut rows: Vec<Vec<String>> = Vec::new();
        let mut max_cols = 0;

        // 遍历所有行，提取单元格内容
        for child in elem.children() {
            if let Some(row) = ElementRef::wrap(child) {
                let tag = &*row.value().name.local;
                if tag == "tr" {
                    let mut cells = Vec::new();
                    for cell in row.children() {
                        if let Some(cell_elem) = ElementRef::wrap(cell) {
                            let cell_tag = &*cell_elem.value().name.local;
                            if cell_tag == "th" || cell_tag == "td" {
                                let inner = self.element_to_markdown(cell_elem);
                                let text = inner.trim().to_string();
                                cells.push(text);
                            }
                        }
                    }
                    max_cols = max_cols.max(cells.len());
                    rows.push(cells);
                }
            }
        }

        if rows.is_empty() || max_cols == 0 {
            return;
        }

        // 补齐所有行到相同列数
        for cells in &mut rows {
            while cells.len() < max_cols {
                cells.push(String::new());
            }
        }

        // 输出表头行
        if let Some(header) = rows.first() {
            let header_line = format!("| {} |", header.join(" | "));
            parts.push(header_line);
            parts.push("\n".to_string());

            // 输出分隔线
            let sep = format!("|{}|", vec!["---"; max_cols].join("|"));
            parts.push(sep);
            parts.push("\n".to_string());
        }

        // 输出数据行
        for row in rows.iter().skip(1) {
            let line = format!("| {} |", row.join(" | "));
            parts.push(line);
            parts.push("\n".to_string());
        }

        parts.push("\n".to_string());
    }

    // ── 文本清理 ──

    /// 合并多余换行和空格
    fn clean_text(&self, text: &str) -> String {
        let text = newlines_regex().replace_all(text, "\n\n");
        let text = spaces_regex().replace_all(&text, " ");
        text.trim().to_string()
    }
}

// ── 工具函数 ──

/// 从 scraper::node::Element 中按属性名取值
fn elem_attr(el: &scraper::node::Element, name: &str) -> Option<String> {
    el.attrs
        .iter()
        .find(|(n, _)| &*n.local == name)
        .map(|(_, v)| v.to_string())
}