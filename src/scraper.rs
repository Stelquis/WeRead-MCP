//! HTTP 请求管理器
//!
//! 负责通过纯 HTTP 请求获取微信文章 HTML、下载图片、写入本地文件。
//! 替代原 Headless Chrome 方案，消除了浏览器依赖。

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use reqwest::Client;
use tokio::fs;
use tokio::sync::Semaphore;

use crate::error::AppError;
use crate::parser::ArticleData;

/// 内存缓存大小上限（最多缓存 50 篇文章）
const CACHE_MAX_SIZE: usize = 50;

/// 缓存 TTL（30 分钟）
const CACHE_TTL: Duration = Duration::from_secs(1800);

/// 缓存条目
struct CacheEntry {
    data: ArticleData,
    cached_at: Instant,
}

/// 图片下载重试次数
const IMAGE_DOWNLOAD_RETRIES: u32 = 2;

/// HTTP 请求管理器
///
/// 使用纯 HTTP 请求获取文章 HTML，无需 Chrome/Chromium 浏览器。
/// 包含一个简单的内存缓存，避免重复请求相同 URL。
pub struct WeixinScraper {
    client: Client,
    cache: std::sync::Mutex<HashMap<String, CacheEntry>>,
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
            client,
            cache: std::sync::Mutex::new(HashMap::new()),
        }
    }

    /// 获取微信文章内容（带缓存 + 重试）
    ///
    /// 先检查内存缓存（超时条目视为未命中），命中则直接返回。
    /// 未命中则发送 HTTP 请求，网络错误和非 200 状态码会触发重试（指数退避: 1s → 3s → 9s）。
    pub async fn fetch_article(&self, url: &str) -> Result<ArticleData, AppError> {
        // 1. 检查缓存（TTL 过期视为未命中）
        {
            let mut cache = self.cache.lock().unwrap();
            if let Some(entry) = cache.get(url) {
                if entry.cached_at.elapsed() < CACHE_TTL {
                    tracing::info!("缓存命中: {}", url);
                    return Ok(entry.data.clone());
                } else {
                    // 缓存过期，移除以节省内存
                    cache.remove(url);
                    tracing::info!("缓存过期: {}", url);
                }
            }
        }

        // 2. 未命中，发起 HTTP 请求（带重试）
        let article = self.fetch_article_http(url).await?;

        // 3. 存入缓存
        {
            let mut cache = self.cache.lock().unwrap();
            // 缓存超过上限时，清空最旧的记录
            if cache.len() >= CACHE_MAX_SIZE {
                cache.clear();
                tracing::info!("缓存已满，已清空");
            }
            cache.insert(
                url.to_string(),
                CacheEntry {
                    data: article.clone(),
                    cached_at: Instant::now(),
                },
            );
        }

        Ok(article)
    }

    /// 实际发送 HTTP 请求获取文章（带重试，最多 3 次，指数退避）
    async fn fetch_article_http(&self, url: &str) -> Result<ArticleData, AppError> {
        let max_retries = 3;
        let mut last_err = None;

        for attempt in 1..=max_retries {
            if attempt > 1 {
                let delay = Duration::from_secs(3u64.pow(attempt as u32 - 2));
                tracing::info!("重试 [{}/{}]: 等待 {:.1}s 后重试...", attempt, max_retries, delay.as_secs_f64());
                tokio::time::sleep(delay).await;
            }

            tracing::info!("正在获取文章 [尝试 {}/{}]: {}", attempt, max_retries, url);

            match self.client.get(url).send().await {
                Ok(response) => {
                    if !response.status().is_success() {
                        let err = AppError::HttpStatusError {
                            status: response.status().as_u16(),
                            message: response.status().canonical_reason().unwrap_or("unknown").to_string(),
                        };
                        tracing::warn!("{}", err);
                        last_err = Some(err);
                        continue;
                    }

                    match response.text().await {
                        Ok(html) => {
                            tracing::info!("获取成功 ({} bytes), 正在解析...", html.len());
                            return Ok(crate::parser::parse(&html));
                        }
                        Err(e) => {
                            let err = AppError::ResponseReadError(format!("{}", e));
                            tracing::warn!("{}", err);
                            last_err = Some(err);
                            continue;
                        }
                    }
                }
                Err(e) => {
                    let err = if e.is_timeout() {
                        AppError::TimeoutError(format!("请求超时: {}", e))
                    } else {
                        AppError::NetworkError(format!("{}", e))
                    };
                    tracing::warn!("{}", err);
                    last_err = Some(err);
                    continue;
                }
            }
        }

        Err(last_err.unwrap_or_else(|| AppError::NetworkError("请求失败: 未知错误".to_string())))
    }

    /// 下载文章中的图片到本地目录（并发下载，最多 5 张同时）
    ///
    /// 遍历图片 URL 列表，并发下载并保存到 `output_dir/images/` 目录。
    /// 使用 Semaphore 限制最大并发数，避免被限流。
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

        // 并发下载，限制最多 5 张同时
        let semaphore = Arc::new(Semaphore::new(5));
        let client = Arc::new(self.client.clone());
        let mut tasks = Vec::new();
        let mut file_idx = 0;

        for (idx, url) in images.iter().enumerate() {
            let url = url.trim().to_string();
            if url.is_empty() {
                tracing::warn!("跳过空图片 URL [{}]", idx);
                continue;
            }

            // 每个任务拿到自己的文件名和路径
            let filename = self.image_filename(&url, file_idx);
            file_idx += 1;
            let save_path = images_dir.join(&filename);
            let permit = Arc::clone(&semaphore);
            let client = Arc::clone(&client);

            tasks.push(tokio::spawn(async move {
                // 获取信号量许可，等待并发槽位
                let _permit = permit.acquire().await.unwrap();

                tracing::info!("Downloading image [{}]: {}", idx + 1, url);

                // 带重试的图片下载
                for retry in 0..IMAGE_DOWNLOAD_RETRIES {
                    if retry > 0 {
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        tracing::info!("图片重试 [{}/{}]: {}", retry, IMAGE_DOWNLOAD_RETRIES - 1, url);
                    }

                    match client.get(&url).send().await {
                        Ok(response) => match response.bytes().await {
                            Ok(bytes) => {
                                if let Err(e) = fs::write(&save_path, &bytes).await {
                                    tracing::warn!("图片保存失败 [{}]: {}", filename, e);
                                    break;
                                }
                                tracing::info!("图片下载成功 [{}]: {}", idx + 1, filename);
                                return Some((url, filename));
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
                None
            }));
        }

        // 等待所有下载任务完成
        let mut url_to_file = HashMap::new();
        let mut success = 0u32;
        let total = tasks.len();
        for task in tasks {
            if let Some((url, filename)) = task.await.unwrap_or(None) {
                url_to_file.insert(url, filename);
                success += 1;
            }
        }

        tracing::info!("下载完成: {}/{} 张图片", success, total);
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
    ///
    /// 当 `wx_fmt` 存在且与 basename 扩展名不一致时，生成 `image_N.ext` 格式
    /// 以避免扩展名不匹配（如 URL 路径为 `.jpg` 但 `wx_fmt=png`）。
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

        // 从 wx_fmt 参数取扩展名（微信图片 URL 常用，优先级最高）
        let wx_fmt_ext = url
            .split('?')
            .nth(1)
            .and_then(|q| {
                q.split('&')
                    .find(|p| p.starts_with("wx_fmt="))
                    .map(|p| &p[7..])
            })
            .filter(|e| KNOWN_EXTS.contains(e));

        // 从 URL 路径取扩展名
        let path_ext = basename
            .rsplit('.')
            .next()
            .filter(|e| KNOWN_EXTS.contains(e));

        // 优先使用 wx_fmt 的扩展名
        let ext = wx_fmt_ext.or(path_ext).unwrap_or("jpg");

        // 决定是否使用 basename 作为文件名
        let use_basename = if wx_fmt_ext.is_some() {
            // 有 wx_fmt：仅当 basename 扩展名与 wx_fmt 一致时才保留原文件名
            basename.contains('.') && path_ext == wx_fmt_ext
        } else {
            // 无 wx_fmt：basename 有扩展名就保留
            basename.contains('.')
        };

        if use_basename {
            basename.to_string()
        } else {
            format!("image_{}.{}", idx, ext)
        }
    }
}

// ── 单元测试 ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_filename_from_wx_fmt() {
        let scraper = WeixinScraper::new();
        let url = "https://mmbiz.qpic.cn/xxx?wx_fmt=png&other=1";
        let name = scraper.image_filename(url, 0);
        assert_eq!(name, "image_0.png");
    }

    #[test]
    fn test_image_filename_from_url_path() {
        let scraper = WeixinScraper::new();
        let url = "https://example.com/image.jpg";
        let name = scraper.image_filename(url, 0);
        assert_eq!(name, "image.jpg");
    }

    #[test]
    fn test_image_filename_unknown_ext_preserves_basename() {
        let scraper = WeixinScraper::new();
        let url = "https://example.com/image.unknown";
        let name = scraper.image_filename(url, 5);
        // 未知扩展名但 basename 自带扩展名，保留原文件名
        assert_eq!(name, "image.unknown");
    }

    #[test]
    fn test_image_filename_no_ext_defaults_to_jpg() {
        let scraper = WeixinScraper::new();
        let url = "https://example.com/image";
        let name = scraper.image_filename(url, 3);
        assert_eq!(name, "image_3.jpg");
    }
}
