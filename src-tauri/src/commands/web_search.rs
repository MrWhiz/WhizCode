use serde::{Deserialize, Serialize};
use scraper::{Html, Selector};
use crate::error::Result;

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

#[tauri::command]
pub async fn search_web(query: String) -> Result<Vec<SearchResult>> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .build()?;

    let url = format!("https://html.duckduckgo.com/html?q={}", urlencoding::encode(&query));
    let response = client.get(url).send().await?.text().await?;
    
    let document = Html::parse_document(&response);
    let result_selector = Selector::parse(".result").unwrap();
    let title_selector = Selector::parse(".result__a").unwrap();
    let snippet_selector = Selector::parse(".result__snippet").unwrap();
    
    let mut results = Vec::new();
    for element in document.select(&result_selector).take(8) {
        if let (Some(title_el), Some(snippet_el)) = (element.select(&title_selector).next(), element.select(&snippet_selector).next()) {
            let title = title_el.text().collect::<String>().trim().to_string();
            let url = title_el.value().attr("href")
                .and_then(|h| h.split("//duckduckgo.com/l/?kh=-1&uddg=").nth(1))
                .and_then(|u| u.split('&').next())
                .map(|u| urlencoding::decode(u).unwrap_or(u.into()).into_owned())
                .unwrap_or_else(|| title_el.value().attr("href").unwrap_or("").to_string());
                
            let snippet = snippet_el.text().collect::<String>().trim().to_string();
            
            if !title.is_empty() && !url.is_empty() {
                results.push(SearchResult { title, url, snippet });
            }
        }
    }
    
    Ok(results)
}

#[tauri::command]
pub async fn read_url_content(url: String) -> Result<String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .build()?;

    let response = client.get(&url).send().await?.text().await?;
    let document = Html::parse_document(&response);
    
    // Extract main text (p, h1, h2, h3, pre, code)
    let content_selector = Selector::parse("main, article, .content, #content, body").unwrap();
    let mut text_output = String::new();
    
    if let Some(main_el) = document.select(&content_selector).next() {
        let tag_selector = Selector::parse("p, h1, h2, h3, h4, li, pre, code").unwrap();
        for el in main_el.select(&tag_selector) {
            let tag_name = el.value().name();
            let text = el.text().collect::<String>().trim().to_string();
            
            if !text.is_empty() {
                match tag_name {
                    "h1" => text_output.push_str(&format!("# {}\n\n", text)),
                    "h2" => text_output.push_str(&format!("## {}\n\n", text)),
                    "h3" => text_output.push_str(&format!("### {}\n\n", text)),
                    "pre" | "code" => text_output.push_str(&format!("```\n{}\n```\n\n", text)),
                    _ => text_output.push_str(&format!("{}\n\n", text)),
                }
            }
        }
    }
    
    if text_output.is_empty() {
        // Fallback: just get all body text
        text_output = document.root_element().text().collect::<Vec<_>>().join(" ");
    }
    
    // Truncate if too long (max 15k chars for prompt safety)
    if text_output.len() > 15000 {
        text_output.truncate(14500);
        text_output.push_str("\n\n[Content truncated due to length...]");
    }
    
    Ok(text_output)
}
