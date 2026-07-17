//! MCP 服务器实现
//!
//! 负责 MCP 工具注册、URL 校验、响应构造、输出目录管理。
//! 通过 `read_weixin_article` 工具对外提供微信公众号文章读取能力。

use rmcp::handler::server::wrapper::Parameters;
use rmcp::{
    handler::server::tool::ToolRouter, model::*, schemars, tool, tool_handler, tool_router,
    ServerHandler,
};
use std::path::Path;

use crate::scraper::WeixinScraper;

/// 工具的请求参数
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ReadArticleRequest {
    #[schemars(description = "微信文章URL，格式: https://mp.weixin.qq.com/s/xxx")]
    pub url: String,

    #[schemars(
        description = "输出目录（可选，默认 ./output/，也可通过环境变量 WEREAD_MCP_OUTPUT_DIR 设置）"
    )]
    pub output_dir: Option<String>,
}

/// 将字符串中的非法文件名字符替换为下划线（保留中文、字母、数字、常用符号）
fn sanitize_filename(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches(&['_', ' '][..])
        .to_string()
}

/// MCP 服务器结构体
#[derive(Clone)]
pub struct WeixinServer {
    scraper: std::sync::Arc<WeixinScraper>,
    tool_router: ToolRouter<Self>,
}

impl WeixinServer {
    pub fn new() -> Self {
        Self {
            scraper: std::sync::Arc::new(WeixinScraper::new()),
            tool_router: Self::tool_router(),
        }
    }
}

/// 工具方法定义
#[tool_router]
impl WeixinServer {
    /// 读取微信公众号文章内容
    #[tool(
        description = "读取微信公众号文章内容。接收一个微信文章 URL，提取结构化内容（标题、作者、正文 Markdown、图片列表）。URL 必须以 https://mp.weixin.qq.com/s/ 开头。自动将文章 Markdown 和图片下载到 ./output/<文章标题>/ 目录下。返回：title, author, publish_time, content(纯文本), content_markdown(带格式), images(图片URL列表), output(本地文件路径信息)。\n\n【AI 处理流程】当你拿到返回的 content_markdown 和 output.markdown_path 后，请按以下步骤处理：\n1. 读取 output.markdown_path 指向的 article.md 文件（包含完整正文和本地图片引用）\n2. 从头到尾通读全文，理解文章结构和内容逻辑\n3. 对 Markdown 进行排版优化：\n   - 确保标题层级合理（H1 文章标题、H2 章节、H3 子章节）\n   - 修正代码块、引用、列表的格式\n   - 合并/拆分过短或过长的段落\n   - 修正图片 alt 文本为空的问题\n4. 对内容进行润色：\n   - 修正明显的中英文标点混用\n   - 统一术语表述\n   - 修复明显的排版错误（多余空格、换行等）\n5. ⚠️ 重要约束：上述排版优化和润色过程中，必须保留全部原始内容，不得删除任何段落、章节、公式、示例或图片引用。只修正排版和语言，不删减信息量。\n6. 将文章的大标题（H1）改为超链接格式，格式为 `# [标题文字](原始URL)`，链接到原始微信文章 URL（即本次调用时传入的 URL）\n7. 输出最终润色后的 Markdown 内容"
    )]
    async fn read_weixin_article(&self, Parameters(req): Parameters<ReadArticleRequest>) -> String {
        let url = req.url;

        // 1. URL 校验
        if !url.starts_with("https://mp.weixin.qq.com/s/") {
            let error_msg = format!(
                "Invalid URL format. Must be a Weixin article URL (https://mp.weixin.qq.com/s/xxx). Got: {}",
                url
            );
            tracing::warn!("{}", error_msg);

            return serde_json::json!({
                "success": false,
                "error": error_msg
            })
            .to_string();
        }

        tracing::info!("Fetching article: {}", url);

        // 2. 调用爬虫获取文章
        match self.scraper.fetch_article(&url).await {
            Ok(article) => {
                tracing::info!("Successfully fetched: {}", article.title);

                let mut response = serde_json::json!({
                    "success": true,
                    "title": article.title,
                    "author": article.author,
                    "publish_time": article.publish_time,
                    "content": article.content,
                    "content_markdown": article.content_markdown,
                    "images": article.images,
                    "error": null
                });

                // 3. 自动输出到目录（优先级: 工具参数 > 环境变量 > 默认 ./output/）
                let base_dir = req
                    .output_dir
                    .filter(|d| !d.is_empty())
                    .or_else(|| {
                        std::env::var("WEREAD_MCP_OUTPUT_DIR")
                            .ok()
                            .filter(|d| !d.is_empty())
                    })
                    .unwrap_or_else(|| "output".to_string());
                let folder_name = sanitize_filename(&article.title);
                let folder_name = if folder_name.is_empty() {
                    "untitled".to_string()
                } else {
                    folder_name.chars().take(80).collect::<String>()
                };
                let output_path = Path::new(&base_dir).join(&folder_name);

                // 创建输出目录
                if let Err(e) = tokio::fs::create_dir_all(&output_path).await {
                    let err_msg = format!("创建输出目录失败: {}", e);
                    tracing::error!("{}", err_msg);
                    response["output"] = serde_json::json!({
                        "success": false,
                        "error": err_msg
                    });
                } else {
                    // 下载图片
                    let dl_result = self
                        .scraper
                        .download_images(&article.images, &output_path)
                        .await;

                    match dl_result {
                        Ok(url_to_file) => {
                            // 写入 Markdown 文件（含本地图片路径）
                            tracing::info!("[STAGE] 写入 Markdown 文件");
                            let md_result = self
                                .scraper
                                .write_article_output(&article, &output_path, &url_to_file)
                                .await;

                            match md_result {
                                Ok(md_path) => {
                                    let downloaded: Vec<String> =
                                        url_to_file.values().cloned().collect();
                                    response["output"] = serde_json::json!({
                                        "success": true,
                                        "markdown_path": md_path,
                                        "images_dir": format!("{}/images", output_path.display()),
                                        "downloaded_images": downloaded,
                                    });
                                    tracing::info!("文章已输出到: {}", md_path);
                                }
                                Err(e) => {
                                    let downloaded: Vec<String> =
                                        url_to_file.values().cloned().collect();
                                    response["output"] = serde_json::json!({
                                        "success": false,
                                        "error": format!("Markdown 写入失败: {}", e),
                                        "downloaded_images": downloaded,
                                    });
                                }
                            }
                        }
                        Err(e) => {
                            response["output"] = serde_json::json!({
                                "success": false,
                                "error": format!("图片下载失败: {}", e),
                            });
                        }
                    }
                }

                response.to_string()
            }
            Err(e) => {
                tracing::error!("Failed to fetch article: {}", e);

                serde_json::json!({
                    "success": false,
                    "error": e.to_string()
                })
                .to_string()
            }
        }
    }
}

/// 实现 MCP 协议处理器
#[tool_handler]
impl ServerHandler for WeixinServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "WeRead MCP 服务 — 微信文章阅读器。提供 read_weixin_article 工具，\
                 可以读取微信公众号文章的标题、作者、发布时间和正文内容（Markdown 格式 + 图片）。\n\n\
                 【工作流程】\n\
                 1. 调用 read_weixin_article 工具，传入微信文章 URL\n\
                 2. 工具返回文章的标题、作者、正文 Markdown、图片列表，同时将 article.md 和图片下载到 ./output/<文章标题>/ 目录\n\
                 3. 读取 output 字段中的 markdown_path 指向的 article.md 文件\n\
                 4. 从头到尾通读全文，理解文章结构和内容逻辑\n\
                 5. 对 Markdown 进行排版优化：\n\
                    - 确保标题层级合理（H1 文章标题、H2 章节、H3 子章节）\n\
                    - 修正代码块、引用、列表的格式\n\
                    - 合并/拆分过短或过长的段落\n\
                    - 修正图片 alt 文本为空的问题\n\
                 6. 对内容进行润色：\n\
                    - 修正中英文标点混用\n\
                    - 统一术语表述\n\
                    - 修复多余空格、换行等排版错误\n\
                 7. ⚠️ 重要约束：上述排版优化和润色过程中，必须保留全部原始内容，不得删除任何段落、章节、公式、示例或图片引用。只修正排版和语言，不删减信息量。\n\
                 8. 将文章的大标题（H1）改为超链接格式，格式为 `# [标题文字](原始URL)`，链接到原始微信文章 URL（即第 1 步传入的 URL）\n\
                 9. 将润色后的内容写回 article.md，输出最终结果"
                    .into(),
            ),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            ..Default::default()
        }
    }
}

// ── 单元测试 ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename_keeps_chinese() {
        assert_eq!(sanitize_filename("一文讲透Skill"), "一文讲透Skill");
    }

    #[test]
    fn test_sanitize_filename_replaces_special_chars() {
        assert_eq!(sanitize_filename("8:00 AI 早报"), "8_00 AI 早报");
    }

    #[test]
    fn test_sanitize_filename_trims_edges() {
        assert_eq!(sanitize_filename("__hello__"), "hello");
    }

    #[test]
    fn test_sanitize_filename_keeps_dot_and_hyphen() {
        assert_eq!(sanitize_filename("hello-world.test"), "hello-world.test");
    }
}
