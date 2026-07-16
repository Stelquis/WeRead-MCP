//! HTTP 请求管理器
//!
//! 负责通过纯 HTTP 请求获取微信文章 HTML、下载图片、写入本地文件。
//! 替代原 Headless Chrome 方案，消除了浏览器依赖。

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use reqwest::Client;
use tokio::fs;

use crate::error::AppError;
use crate::parser::{ArticleData, WeixinParser};

/// HTTP 请求管理器
///
/// 使用纯 HTTP 请求获取文章 HTML，无需 Chrome/Chromium 浏览器。
/// 优点：
/// - 无环境依赖，启动毫秒级
/// - 资源占用低（几十 KB vs 几百 MB）
/// - 避免被微信反爬检测（浏览器指纹反而更容易被封）
pub struct WeixinScraper {
    parser: WeixinParser,
    client: Client,
}

impl WeixinScraper {
    /// 创建 HTTP 请求管理器
    ///
    /// 初始化 HTTP 客户端，配置请求头以模拟微信内置浏览器访问。
    pub fn new() -> Self {
        let mut default_headers = reqwest::header::HeaderMap::new();

        // 设置 Accept 头，接受 HTML 和图片资源
        default_headers.insert(
            reqwest::header::ACCEPT,
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8"
                .parse()
                .unwrap(),
        );
        // 设置语言偏好，优先中文
        default_headers.insert(
            reqwest::header::ACCEPT_LANGUAGE,
            "zh-CN,zh;q=0.9,en;q=0.8".parse().unwrap(),
        );
        // 设置 Referer 模拟从微信内打开，绕过反爬检测
        default_headers.insert(
            reqwest::header::REFERER,
            "https://mp.weixin.qq.com/".parse().unwrap(),
        );

        // 构建 HTTP 客户端，配置超时和 User-Agent
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            )
            .default_headers(default_headers)
            .build()
            .expect("Failed to build HTTP client");

        Self {
            parser: WeixinParser::new(),
            client,
        }
    }

    /// 获取微信文章内容
    ///
    /// 发送 HTTP GET 请求获取文章 HTML，交由解析器提取结构化数据。
    pub async fn fetch_article(&self, url: &str) -> Result<ArticleData, AppError> {
        tracing::info!("Fetching article via HTTP: {}", url);

        // 发送 HTTP 请求
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| AppError::HttpError(format!("Request failed: {}", e)))?;

        // 检查 HTTP 状态码
        if !response.status().is_success() {
            return Err(AppError::HttpError(format!(
                "HTTP {}: {}",
                response.status().as_u16(),
                response.status().canonical_reason().unwrap_or("unknown")
            )));
        }

        // 读取响应体
        let html = response
            .text()
            .await
            .map_err(|e| AppError::HttpError(format!("Read body failed: {}", e)))?;

        tracing::info!("Got HTML ({} bytes), parsing...", html.len());
        Ok(self.parser.parse(&html))
    }

    /// 下载文章中的图片到本地目录
    ///
    /// 遍历图片 URL 列表，逐个下载并保存到 `output_dir/images/` 目录。
    /// 返回 URL → 本地文件名的映射，用于后续替换 Markdown 中的图片引用。
    /// 遇到下载失败的图片会跳过并记录日志，不会中断整体流程。
    pub async fn download_images(
        &self,
        images: &[String],
        output_dir: &Path,
    ) -> Result<HashMap<String, String>, AppError> {
        if images.is_empty() {
            return Ok(HashMap::new());
        }

        // 创建图片存储目录
        let images_dir = output_dir.join("images");
        fs::create_dir_all(&images_dir)
            .await
            .map_err(|e| AppError::IoError(format!("创建图片目录失败: {}", e)))?;

        let mut url_to_file = HashMap::new();
        let mut file_idx = 0;

        for (idx, url) in images.iter().enumerate() {
            // 跳过空 URL 和无效 URL
            let url = url.trim();
            if url.is_empty() {
                tracing::warn!("跳过空图片 URL [{}]", idx);
                continue;
            }

            // 从 URL 推导文件名，使用连续编号避免跳号
            let filename = self.image_filename(url, file_idx);
            file_idx += 1;
            let save_path = images_dir.join(&filename);

            tracing::info!("Downloading image [{}/{}]: {}", idx + 1, images.len(), url);

            // 发送 HTTP 请求下载图片
            match self.client.get(url).send().await {
                Ok(response) => match response.bytes().await {
                    Ok(bytes) => {
                        // 写入本地文件
                        if let Err(e) = fs::write(&save_path, &bytes).await {
                            tracing::warn!("图片保存失败 [{}]: {}", filename, e);
                            continue;
                        }
                        url_to_file.insert(url.to_string(), filename);
                    }
                    Err(e) => {
                        tracing::warn!("图片读取失败 [{}]: {}", url, e);
                        continue;
                    }
                },
                Err(e) => {
                    tracing::warn!("图片下载失败 [{}]: {}", url, e);
                    continue;
                }
            }
        }

        tracing::info!("下载完成: {}/{} 张图片", url_to_file.len(), images.len());
        Ok(url_to_file)
    }

    /// 将文章 Markdown 写入本地文件，并将图片引用替换为本地路径
    ///
    /// 遍历 `url_to_file` 映射，将 Markdown 中的远程图片 URL 替换为 `images/xxx` 本地路径。
    /// 返回写入的 markdown 文件绝对路径。
    pub async fn write_article_output(
        &self,
        article: &ArticleData,
        output_dir: &Path,
        url_to_file: &HashMap<String, String>,
    ) -> Result<String, AppError> {
        // 替换 Markdown 中的图片引用为本地路径
        let mut md = article.content_markdown.clone();
        for (url, local_name) in url_to_file {
            let local_path = format!("images/{}", local_name);
            md = md.replace(url, &local_path);
        }

        // 写入 article.md 文件
        let md_path = output_dir.join("article.md");
        fs::write(&md_path, &md)
            .await
            .map_err(|e| AppError::IoError(format!("Markdown 写入失败: {}", e)))?;

        tracing::info!("Markdown 已保存: {}", md_path.display());
        Ok(md_path.to_string_lossy().to_string())
    }

    /// 从图片 URL 提取合理的本地文件名
    ///
    /// 优先从 `wx_fmt` 参数取扩展名，兜底从 URL 路径取。
    /// 只识别已知的图片格式（jpg/jpeg/png/gif/webp/bmp/svg/ico），
    /// 未知格式或 `other` 时默认使用 `jpg`。
    fn image_filename(&self, url: &str, idx: usize) -> String {
        // 去掉 query string，取最后一段路径作为 basename
        let url_path = url.split('?').next().unwrap_or(url);
        let basename = url_path
            .rsplit('/')
            .next()
            .filter(|s| !s.is_empty())
            .unwrap_or("image");

        // 已知的图片扩展名白名单
        const KNOWN_EXTS: &[&str] = &["jpg", "jpeg", "png", "gif", "webp", "bmp", "svg", "ico"];

        // 优先从 wx_fmt 参数取扩展名，不在白名单则从 URL 路径取
        let ext = url
            .split('?')
            .nth(1)
            .and_then(|q| {
                q.split('&')
                    .find(|p| p.starts_with("wx_fmt="))
                    .map(|p| &p[7..])
            })
            .filter(|e| KNOWN_EXTS.contains(e))
            .or_else(|| {
                basename
                    .rsplit('.')
                    .next()
                    .filter(|e| KNOWN_EXTS.contains(e))
            })
            .unwrap_or("jpg");

        // basename 自带扩展名则直接使用，否则生成 image_N.ext 格式
        if basename.contains('.') && ext.len() <= 5 {
            basename.to_string()
        } else {
            format!("image_{}.{}", idx, ext)
        }
    }
}
