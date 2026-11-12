//! Official Minecraft headlines and artwork for Valtic's home screen.
//! Keep this independent of GPUI: networking, disk I/O and decoding run on a worker.
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;

const FEED: &str = "https://www.minecraft.net/en-us/feeds/community-content/rss";
pub const NEWS_PAGE: &str = "https://www.minecraft.net/en-us/article";
pub const REFRESH_SECS: u64 = 45 * 60;
const MAX_FEED: u64 = 8 * 1024 * 1024;
const MAX_IMAGE: u64 = 6 * 1024 * 1024;

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
        .timeout_connect(Duration::from_secs(12))
        .timeout_read(Duration::from_secs(18))
        .timeout_write(Duration::from_secs(12))
        .build()
}

fn get_bytes(agent: &ureq::Agent, url: &str, cap: u64) -> Option<Vec<u8>> {
    let response = agent.get(url)
        .set("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ValticLauncher/1.0")
        .set("Accept", "application/rss+xml, application/atom+xml, text/html, application/xml, */*")
        .set("Accept-Language", "en-US,en;q=0.9")
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


/// Minecraft's news page can change its RSS provider without warning.
/// Use its official article listing as a second source, never made-up headlines.
const LISTINGS: [&str; 2] = [
    "https://www.minecraft.net/en-us/articles?page=%7B%22g2%22%3A1%7D&sort=%7B%22g2%22%3A%22Recent%22%7D",
    "https://www.minecraft.net/en-us/article",
];

fn article_address(value: &str) -> Option<String> {
    let url = image_address(value)?;
    if !official_article(&url) || !url.split('?').next()?.contains("/article/") {
        return None;
    }
    Some(url)
}

/// News feeds sometimes switch from RSS items to Atom entries.
fn parse_atom(xml: &str) -> Vec<Article> {
    let mut results = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<entry") {
        let opened = &rest[start..];
        let Some(open_end) = opened.find('>') else { break };
        let data = &opened[open_end + 1..];
        let Some(end) = data.find("</entry>") else { break };
        let entry = &data[..end];
        rest = &data[end + "</entry>".len()..];
        let title = tag_value(entry, "title").map(|t| plain_summary(&t)).unwrap_or_default();
        let Some(link_start) = entry.find("<link") else { continue };
        let Some(url) = attribute(&entry[link_start..], "href").and_then(|u| article_address(&u)) else { continue };
        if title.is_empty() { continue; }
        let summary = tag_value(entry, "summary")
            .or_else(|| tag_value(entry, "content"))
            .map(|s| plain_summary(&s))
            .unwrap_or_default();
        results.push(Article { title, summary, url, image_url: first_image(entry) });
        if results.len() >= 3 { break; }
    }
    results
}

fn html_tag_body<'a>(html: &'a str, name: &str) -> Option<&'a str> {
    let start = html.find(&format!("<{name}"))?;
    let opened = &html[start..];
    let content = &opened[opened.find('>')? + 1..];
    Some(&content[..content.find(&format!("</{name}>"))?])
}

fn listing_image(fragment: &str) -> Option<String> {
    for tag in ["img", "source"] {
        if let Some(start) = fragment.find(&format!("<{tag}")) {
            let element = &fragment[start..];
            for name in ["src", "data-src", "srcset", "data-srcset"] {
                if let Some(value) = attribute(element, name) {
                    // A srcset can contain sizes after the first URL.
                    let first = value.split(',').next().unwrap_or_default();
                    let address = first.split_whitespace().next().unwrap_or_default();
                    if let Some(url) = image_address(address) { return Some(url); }
                }
            }
        }
    }
    None
}

/// Read links, titles and (when embedded) images from Minecraft's own latest-news page.
/// A listing must provide an actual article link, not merely promotional text.
fn parse_listing(html: &str) -> Vec<Article> {
    let mut found = Vec::new();
    for anchor in html.split("<a").skip(1) {
        let Some(open_end) = anchor.find('>') else { continue };
        let Some(close) = anchor[open_end + 1..].find("</a>") else { continue };
        let body = &anchor[open_end + 1 .. open_end + 1 + close];
        if body.len() > 20_000 { continue; }
        let Some(url) = attribute(anchor, "href").and_then(|href| article_address(&href)) else { continue };
        if found.iter().any(|old: &Article| old.url == url) { continue; }
        let title = ["h2", "h3", "h4", "h5"]
            .iter().find_map(|name| html_tag_body(body, name))
            .map(plain_summary)
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| plain_summary(body));
        if title.is_empty() || title.len() < 5 || title.len() > 160
            || title.eq_ignore_ascii_case("Read more")
            || title.eq_ignore_ascii_case("Learn more") {
            continue;
        }
        let summary = html_tag_body(body, "p")
            .map(plain_summary).unwrap_or_default();
        found.push(Article { title, summary, url, image_url: listing_image(body) });
        if found.len() >= 3 { break; }
    }
    found
}

fn official_headlines(agent: &ureq::Agent) -> Vec<Article> {
    let mut articles = Vec::new();
    if let Some(data) = get_bytes(agent, FEED, MAX_FEED) {
        if let Ok(feed) = String::from_utf8(data) {
            articles = parse_rss(&feed);
            if articles.is_empty() { articles = parse_atom(&feed); }
        }
    }
    if articles.len() < 3 {
        for listing in LISTINGS {
            let Some(data) = get_bytes(agent, listing, MAX_FEED) else { continue };
            let Ok(html) = String::from_utf8(data) else { continue };
            for article in parse_listing(&html) {
                if !articles.iter().any(|old| old.url == article.url) {
                    articles.push(article);
                }
                if articles.len() >= 3 { break; }
            }
            if articles.len() >= 3 { break; }
        }
    }
    articles.truncate(3);
    articles
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
    let mut articles = official_headlines(&agent);
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
    fn reads_official_html_article_cards() {
        let html = r#"<a href="/en-us/article/minecraft-26-4-snapshot-8"><img src="/content/dam/cover.webp"><h3>Minecraft 26.4 Snapshot 8</h3><p>What's new in testing</p></a>"#;
        let cards = parse_listing(html);
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].title, "Minecraft 26.4 Snapshot 8");
        assert_eq!(cards[0].image_url.as_deref(), Some("https://www.minecraft.net/content/dam/cover.webp"));
    }

    #[test]
    fn reads_official_atom_entries() {
        let feed = r#"<feed><entry><title>Minecraft 26.4 Snapshot 8</title><link href="https://www.minecraft.net/en-us/article/minecraft-26-4-snapshot-8"/></entry></feed>"#;
        let cards = parse_atom(feed);
        assert_eq!(cards.len(), 1);
    }

    #[test]
    fn ignores_unofficial_links() {
        let feed = "<item><title>Fake story</title><link>https://example.org/article</link></item>";
        assert!(parse_rss(feed).is_empty());
    }
}
