use anyhow::Result;
use image::{ImageBuffer, Rgba};
use imageproc::drawing::draw_hollow_rect_mut;
use imageproc::rect::Rect;
use scraper::{Html, Selector};
use zone_core::{BoundingBox, DomSnapshot, EngineMode, InteractiveElement};

pub struct PureEngine {
    client: reqwest::Client,
    viewport_width: u32,
    viewport_height: u32,
}

impl PureEngine {
    pub fn new(viewport_width: u32, viewport_height: u32) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36 ZoneBrowser/0.1")
            .build()
            .unwrap_or_default();

        Self {
            client,
            viewport_width,
            viewport_height,
        }
    }

    /// Fetches HTML from URL and parses interactive elements
    pub async fn load_and_parse(&self, url: &str) -> Result<(DomSnapshot, bool)> {
        let resp = self.client.get(url).send().await?;
        let status = resp.status();
        
        // Anti-bot heuristic: 403, 429 -> trigger Gear Shift to WebView2
        if status == reqwest::StatusCode::FORBIDDEN || status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Ok((
                DomSnapshot {
                    url: url.to_string(),
                    title: "Access Restricted / WAF".to_string(),
                    engine_used: EngineMode::PureRust,
                    elements: vec![],
                },
                true, // needs_escalation
            ));
        }

        let html_text = resp.text().await?;
        let document = Html::parse_document(&html_text);

        // Title extraction
        let title_selector = Selector::parse("title").unwrap();
        let title = document
            .select(&title_selector)
            .next()
            .map(|el| el.text().collect::<Vec<_>>().join(" ").trim().to_string())
            .unwrap_or_else(|| "Untitled Page".to_string());

        // Check for Cloudflare / Turnstile markers in title or body
        if title.contains("Just a moment...") || html_text.contains("cf-turnstile") {
            return Ok((
                DomSnapshot {
                    url: url.to_string(),
                    title,
                    engine_used: EngineMode::PureRust,
                    elements: vec![],
                },
                true, // needs_escalation
            ));
        }

        // Extract interactive elements
        let interactive_selector = Selector::parse(
            "a[href], button, input, select, textarea, [role='button'], [role='link']",
        )
        .unwrap();

        let mut elements = Vec::new();
        let mut curr_y = 40.0;
        let mut id_counter = 0;

        for element in document.select(&interactive_selector) {
            let tag = element.value().name().to_string();
            let role = element
                .value()
                .attr("role")
                .unwrap_or_else(|| match tag.as_str() {
                    "a" => "link",
                    "button" => "button",
                    "input" => "textbox",
                    "select" => "combobox",
                    "textarea" => "textbox",
                    _ => "generic",
                })
                .to_string();

            let text = element
                .text()
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();
            let placeholder = element.value().attr("placeholder").map(|s| s.to_string());

            let text_label = if !text.is_empty() {
                text
            } else if let Some(ref p) = placeholder {
                p.clone()
            } else if let Some(val) = element.value().attr("value") {
                val.to_string()
            } else if let Some(aria) = element.value().attr("aria-label") {
                aria.to_string()
            } else {
                String::new()
            };

            // Estimate visual box placement if running without headless DOM geometry
            let box_width = (160.0f32).max((text_label.len() as f32 * 9.0).min(400.0));
            let box_height = if tag == "textarea" { 80.0 } else { 32.0 };

            let bounds = BoundingBox {
                x: 30.0,
                y: curr_y,
                width: box_width,
                height: box_height,
            };

            curr_y += box_height + 12.0;

            elements.push(InteractiveElement {
                id: id_counter,
                tag,
                role,
                text: text_label,
                placeholder,
                bounds,
            });

            id_counter += 1;
        }

        // Empty SPA shell heuristic: e.g. <div id="root"> with no actionable buttons
        let needs_escalation = elements.len() < 3 && (html_text.contains("id=\"root\"") || html_text.contains("id=\"__next\""));

        Ok((
            DomSnapshot {
                url: url.to_string(),
                title,
                engine_used: EngineMode::PureRust,
                elements,
            },
            needs_escalation,
        ))
    }

    /// Generates a synthesized wireframe canvas with Set-of-Marks badges (<2ms)
    pub fn render_wireframe_som(&self, snapshot: &DomSnapshot) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        let width = self.viewport_width;
        let height = self.viewport_height;

        // Dark background for modern look
        let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_pixel(width, height, Rgba([24, 25, 28, 255]));

        let box_border_color = Rgba([70, 75, 85, 255]);
        let badge_bg_color = Rgba([230, 80, 50, 255]); // high-contrast orange badge

        for el in &snapshot.elements {
            if el.bounds.y + el.bounds.height > height as f32 {
                continue;
            }

            let rx = el.bounds.x.max(0.0) as i32;
            let ry = el.bounds.y.max(0.0) as i32;
            let rw = el.bounds.width as u32;
            let rh = el.bounds.height as u32;

            if rw > 0 && rh > 0 {
                // Draw element bounding box
                draw_hollow_rect_mut(
                    &mut img,
                    Rect::at(rx, ry).of_size(rw, rh),
                    box_border_color,
                );

                // Draw Set-of-Marks index badge on top-left corner: [0], [1], ...
                let badge_w = 28u32;
                let badge_h = 18u32;
                draw_hollow_rect_mut(
                    &mut img,
                    Rect::at(rx - 4, ry - 6).of_size(badge_w, badge_h),
                    badge_bg_color,
                );
            }
        }

        img
    }
}
