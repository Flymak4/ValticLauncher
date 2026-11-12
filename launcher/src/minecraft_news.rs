//! Official Minecraft headlines and artwork for Valtic's home screen.
//! Keep this independent of GPUI: networking, disk I/O and decoding run on a worker.
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;

const FEED: &str = "https://www.minecraft.net/en-us/feeds/community-content/rss";
pub const NEWS_PAGE: &str = "https://www.minecraft.net/en-us/article";
pub const REFRESH_SECS: u64 = 45 * 60;
const MAX_FEED: u64 = 2 * 1024 * 1024;
const MAX_IMAGE: u64 = 3 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Article {
    pub title: String,
    pub summary: String,
    pub url: String,
    pub image_url: Option<String>,
}

fn directory() -> PathBuf {
    horizon_launcher::backend::data_dir().join("cache").join("minecraft-news")
}

fn cache_file() -> PathBuf {
    directory().join("articles.json")
}

/// Read only the most recently fetched official articles. Never show invented headlines.
pub fn load_cached() -> Vec<Article> {
    fs::read(cache_file())
        .ok()
        .and_then(|data| serde_json::from_slice::<Vec<Article>>(&data).ok())
        .unwrap_or_default()
        .into_iter()
        .filter(|article| official_article(&article.url))
        .take(3)
        .collect()
}

fn official_article(url: &str) -> bool {
    url.starts_with("https://www.minecraft.net/")
        || url.starts_with("https://minecraft.net/")
}

fn client() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(6))
        .timeout_read(Duration::from_secs(8))
        .timeout_write(Duration::from_secs(6))
        .build()
}

fn get_bytes(agent: &ureq::Agent, url: &str, cap: u64) -> Option<Vec<u8>> {
    let response = agent.get(url)
        .set("User-Agent", "ValticLauncher/1.0 (Minecraft news reader)")
        .set("Accept", "*/*")
        .call()
        .ok()?;
    let mut bytes = Vec::new();
    response.into_reader().take(cap + 1).read_to_end(&mut bytes).ok()?;
    (bytes.len() as u64 <= cap).then_some(bytes)
}

fn clean(value: &str) -> String {
    let content = value.trim()
        .strip_prefix("<![CDATA[")
        .and_then(|v| v.strip_suffix("]]>"))
        .unwrap_or(value.trim());
    content.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
}

fn tag_value(xml: &str, tag: &str) -> Option<String> {
    let start = xml.find(&format!("<{tag}"))?;
    let body = start + xml[start..].find('>')? + 1;
    let end = body + xml[body..].find(&format!("</{tag}>"))?;
    Some(clean(&xml[body..end]))
}

fn attribute(fragment: &str, name: &str) -> Option<String> {
    // Only scan the opening tag, never text or child elements.
    let head = fragment.split('>').next()?;
    let at = head.find(&format!("{name}="))? + name.len() + 1;
    let quote = head[at..].chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let begin = at + quote.len_utf8();
    let end = begin + head[begin..].find(quote)?;
    Some(clean(&head[begin..end]))
}

fn image_address(input: &str) -> Option<String> {
    let trimmed = input.trim();
    let url = if trimmed.starts_with("https://") {
        trimmed.to_owned()
    } else if trimmed.starts_with("//") {
        format!("https:{trimmed}")
    } else if trimmed.starts_with('/') {
        format!("https://www.minecraft.net{trimmed}")
    } else {
        return None;
    };
    Some(url)
}

fn first_image(item: &str) -> Option<String> {
    for tag in ["media:content", "media:thumbnail", "enclosure"] {
        if let Some(pos) = item.find(&format!("<{tag}")) {
            if let Some(url) = attribute(&item[pos..], "url").and_then(|u| image_address(&u)) {
                return Some(url);
            }
        }
    }
    let description = tag_value(item, "description").unwrap_or_default();
    if let Some(pos) = description.find("<img") {
        return attribute(&description[pos..], "src").and_then(|u| image_address(&u));
    }
    None
}

fn plain_summary(source: &str) -> String {
    let mut plain = String::new();
    let mut markup = false;
    for c in clean(source).chars() {
        if c == '<' {
            markup = true;
        } else if c == '>' {
            markup = false;
            plain.push(' ');
        } else if !markup {
            plain.push(c);
        }
    }
    let condensed = plain.split_whitespace().collect::<Vec<_>>().join(" ");
    condensed.chars().take(160).collect()
}

/// Recognizes the standard RSS <item> structure without a heavyweight XML dependency.
fn parse_rss(xml: &str) -> Vec<Article> {
    let mut result = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<item") {
        let opened = &rest[start..];
        let Some(open_end) = opened.find('>') else { break };
        let data = &opened[open_end + 1..];
        let Some(end) = data.find("</item>") else { break };
        let item = &data[..end];
        rest = &data[end + "</item>".len()..];
        let Some(title) = tag_value(item, "title") else { continue };
        let Some(url) = tag_value(item, "link") else { continue };
        let title = plain_summary(&title);
        let url = url.trim().to_owned();
        if title.is_empty() || !official_article(&url) {
            continue;
        }
        let summary = tag_value(item, "description").map_or_else(String::new, |s| plain_summary(&s));
        result.push(Article { title, summary, url, image_url: first_image(item) });
        if result.len() == 3 { break; }
    }
    result
}

fn page_image(agent: &ureq::Agent, article: &str) -> Option<String> {
    let html = String::from_utf8(get_bytes(agent, article, MAX_FEED)?).ok()?;
    // Article HTML usually declares the official hero artwork as Open Graph metadata.
    for meta in html.split("<meta").skip(1).take(350) {
        let tag = meta.split('>').next().unwrap_or_default();
        if tag.contains("og:image") || tag.contains("twitter:image") {
            if let Some(url) = attribute(tag, "content").and_then(|u| image_address(&u)) {
                return Some(url);
            }
        }
    }
    None
}

fn image_filename(url: &str) -> PathBuf {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(url.as_bytes());
    let id: String = digest.iter().take(12).map(|b| format!("{b:02x}")).collect();
    directory().join(format!("{id}.img"))
}

pub fn cached_image(url: &str) -> Option<Vec<u8>> {
    fs::read(image_filename(url)).ok()
}

fn cache_image(agent: &ureq::Agent, url: &str) {
    let destination = image_filename(url);
    if destination.metadata().is_ok_and(|m| m.len() > 128) { return; }
    let Some(bytes) = get_bytes(agent, url, MAX_IMAGE) else { return };
    // Only save standard image formats; never feed article HTML to the decoder.
    let valid = bytes.starts_with(&[0xFF, 0xD8, 0xFF])
        || bytes.starts_with(b"\x89PNG\r\n\x1a\n")
        || (bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP"))
        || bytes.starts_with(b"GIF8");
    if valid && bytes.len() > 128 {
        let _ = fs::create_dir_all(directory());
        let _ = fs::write(destination, bytes);
    }
}

/// Fetch, validate, cache official headlines and their artwork on a background thread.
/// If Minecraft is offline or changes its feed, the UI retains its previous cached cards.
pub fn refresh() -> Option<Vec<Article>> {
    let agent = client();
    let xml = String::from_utf8(get_bytes(&agent, FEED, MAX_FEED)?).ok()?;
    let mut articles = parse_rss(&xml);
    if articles.is_empty() { return None; }
    for article in &mut articles {
        if article.image_url.is_none() {
            article.image_url = page_image(&agent, &article.url);
        }
        if let Some(url) = article.image_url.as_deref() {
            cache_image(&agent, url);
        }
    }
    if fs::create_dir_all(directory()).is_ok() {
        if let Ok(json) = serde_json::to_vec(&articles) {
            let _ = fs::write(cache_file(), json);
        }
    }
    Some(articles)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_official_articles_and_images() {
        let feed = r#"<rss><channel><item><title><![CDATA[Minecraft 26.4 Snapshot 6]]></title>
        <link>https://www.minecraft.net/en-us/article/snapshot-6</link>
        <description><![CDATA[<p>New blocks &amp; mobs</p>]]></description>
        <media:content url="https://www.minecraft.net/content/dam/cover.jpg" />
        </item></channel></rss>"#;
        let cards = parse_rss(feed);
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].title, "Minecraft 26.4 Snapshot 6");
        assert_eq!(cards[0].summary, "New blocks & mobs");
        assert_eq!(cards[0].image_url.as_deref(),
            Some("https://www.minecraft.net/content/dam/cover.jpg"));
    }

    #[test]
    fn ignores_unofficial_links() {
        let feed = "<item><title>Fake story</title><link>https://example.org/article</link></item>";
        assert!(parse_rss(feed).is_empty());
    }
}
