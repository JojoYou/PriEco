// Set global allovator
#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

use image::GenericImageView;
use ort::{Environment, GraphOptimizationLevel, InMemorySession, LoggingLevel, SessionBuilder};
use reqwest::Client;
use rocket::{
    Request, Response, State,
    fairing::{Fairing, Info, Kind},
    fs::{FileServer, NamedFile},
    get, head,
    http::{ContentType, CookieJar, Header, Status},
    launch, post,
    request::{FromRequest, Outcome},
    response::{
        self, Responder,
        content::{RawJavaScript, RawText},
    },
    routes,
    serde::json::{Json, Value as RocketValue},
};
use rocket_dyn_templates::{Template, handlebars::Handlebars};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    io::{Cursor, Read},
    net::IpAddr,
    path::Path,
    process::exit,
    sync::Arc,
};
use tokenizers::{PaddingDirection, PaddingParams, PaddingStrategy, Tokenizer};
use tokio::sync::Mutex;

// Import local libraries
use prieco_rs::{
    functions::{search_db, search_endpoint, settings},
    globals::{
        CSS_VERSION, EmbeddingService, JS_VERSION, VECTOR_EMBEDDING_MODEL,
        VECTOR_EMBEDDING_TOKENIZER, colors,
    },
    is_valid_url,
};

// Response headers
pub struct GlobalHeaders;
#[rocket::async_trait]
impl Fairing for GlobalHeaders {
    fn info(&self) -> Info {
        Info {
            name: "CORS + Security Headers + Block Bots",
            kind: Kind::Response | Kind::Request,
        }
    }

    async fn on_response<'r>(&self, req: &'r Request<'_>, res: &mut Response<'r>) {
        // --- Block curl/wget ---
        if let Some(agent) = req.headers().get_one("User-Agent") {
            let agent_lower = agent.to_ascii_lowercase();
            if agent_lower.starts_with("wget") || agent_lower.starts_with("curl") {
                res.set_status(Status::Forbidden);
                res.set_sized_body(0, std::io::Cursor::new("")); // empty body
                return; // skip adding headers
            }
        }

        // --- CORS headers ---
        res.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        res.set_header(Header::new("Access-Control-Allow-Methods", "GET"));
        res.set_header(Header::new(
            "Access-Control-Allow-Headers",
            "Content-Type, Authorization",
        ));

        // --- Security headers ---
        res.set_header(Header::new(
            "Content-Security-Policy",
            "default-src 'self'; \
             script-src 'self'; \
             style-src 'self'; \
             img-src 'self' data: https://proxy.prieco.net; \
             connect-src 'self' https://proxy.prieco.net; \
             frame-src 'self'; \
             frame-ancestors 'self'; \
             form-action 'self'; \
             object-src 'none'; \
             base-uri 'self';",
        ));
        res.set_header(Header::new("X-Frame-Options", "SAMEORIGIN"));
        res.set_header(Header::new("X-Content-Type-Options", "nosniff"));
        res.set_header(Header::new("Referrer-Policy", "no-referrer"));
    }
}

// Get user IP address
pub struct ClientIp(pub IpAddr);
#[rocket::async_trait]
impl<'r> FromRequest<'r> for ClientIp {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // Check CF-Connecting-IP header first (Cloudflare)
        if let Some(cf_ip) = req.headers().get_one("CF-Connecting-IP") {
            if let Ok(ip) = cf_ip.parse::<IpAddr>() {
                return Outcome::Success(ClientIp(ip));
            }
        }

        // Check X-Forwarded-For header (other proxies)
        if let Some(xff) = req.headers().get_one("X-Forwarded-For") {
            if let Some(ip_str) = xff.split(',').next() {
                if let Ok(ip) = ip_str.trim().parse::<IpAddr>() {
                    return Outcome::Success(ClientIp(ip));
                }
            }
        }

        // Check X-Real-IP header
        if let Some(real_ip) = req.headers().get_one("X-Real-IP") {
            if let Ok(ip) = real_ip.parse::<IpAddr>() {
                return Outcome::Success(ClientIp(ip));
            }
        }

        // Fall back to socket address
        if let Some(socket_addr) = req.remote() {
            return Outcome::Success(ClientIp(socket_addr.ip()));
        }

        Outcome::Error((rocket::http::Status::BadRequest, ()))
    }
}

// Create embeder + Launch Rocket
#[launch]
fn rocket() -> _ {
    /*
      Vector Embeding model
    */
    let mut tokenizer = match Tokenizer::from_bytes(VECTOR_EMBEDDING_TOKENIZER) {
        Ok(tokenizer) => tokenizer,
        Err(e) => {
            println!(
                "{}Failed to create tokenizer: {}{}",
                colors::RED,
                e,
                colors::RESET
            );
            exit(1);
        }
    };
    tokenizer.with_padding(Some(PaddingParams {
        strategy: PaddingStrategy::BatchLongest,
        direction: PaddingDirection::Right,
        pad_to_multiple_of: None,
        pad_id: 0,
        pad_type_id: 0,
        pad_token: "[PAD]".into(),
    }));

    let environment: Arc<Environment> = match Environment::builder()
        .with_name("embedder")
        .with_log_level(LoggingLevel::Warning)
        .build()
    {
        Ok(env) => Arc::new(env),
        Err(e) => {
            println!(
                "{}Failed to create vector embedding environment: {}{}",
                colors::RED,
                e,
                colors::RESET
            );
            exit(1);
        }
    };

    let mut session_builder = match SessionBuilder::new(&environment) {
        Ok(builder) => builder,
        Err(e) => {
            println!(
                "{}3: Failed to create vector embedding session builder: {}{}",
                colors::RED,
                e,
                colors::RESET
            );
            exit(1);
        }
    };
    session_builder = match session_builder.with_optimization_level(GraphOptimizationLevel::Level3)
    {
        Ok(builder) => builder,
        Err(e) => {
            println!(
                "{}4: Failed to create vector embedding session builder: {}{}",
                colors::RED,
                e,
                colors::RESET
            );
            exit(1);
        }
    };

    session_builder = match session_builder.with_parallel_execution(true) {
        Ok(builder) => builder,
        Err(e) => {
            println!(
                "{}5: Failed to create vector embedding session builder: {}{}",
                colors::RED,
                e,
                colors::RESET
            );
            exit(1);
        }
    };

    let embed_model: InMemorySession =
        match session_builder.with_model_from_memory(VECTOR_EMBEDDING_MODEL) {
            Ok(emb) => emb,
            Err(e) => {
                println!(
                    "{}Failed to create embeder: {}{}",
                    colors::RED,
                    e,
                    colors::RESET
                );
                exit(1);
            }
        };

    // Create the embedding service
    let embedding_service = EmbeddingService {
        tokenizer: Arc::new(tokio::sync::Mutex::new(tokenizer)),
        model: Arc::new(Mutex::new(embed_model)),
    };

    rocket::build()
        .configure(
            rocket::Config::figment()
                .merge(("port", 8081))
                .merge(("address", "127.0.0.1"))
                .merge(("workers", num_cpus::get() * 2)),
        )
        .manage(embedding_service)
        .attach(GlobalHeaders)
        .attach(Template::fairing())
        .mount(
            "/",
            routes![
                // Assets
                sw_js, // Service worker (Browser cache + unduck)
                unduck_js,
                security_txt, // Security.txt
                robots_txt,   // Robots.txt
                script,
                favicon,
                privacy, // Privacy Policy
                // Landing page
                index,
                index_head,
                // Search
                search,
                results_htmls,
                api,
                // Settings
                settings_htmls,
                // Proxy
                proxy_get,
                proxy_post,
            ],
        )
        .mount("/static", FileServer::from("./static"))
}

////
// Assets
////
// Changed route to Root
#[get("/sw.js")]
async fn sw_js() -> Option<NamedFile> {
    NamedFile::open(Path::new("static/js/unduck/sw.js"))
        .await
        .ok()
}
#[get("/unduck.js")]
async fn unduck_js() -> Option<NamedFile> {
    NamedFile::open(Path::new("static/js/unduck/unduck.js"))
        .await
        .ok()
}
#[get("/.well-known/security.txt")]
async fn security_txt() -> RawText<&'static str> {
    RawText(
        "Contact: mailto:support@jojoyou.org\nExpires: 2026-04-16T12:00:00.000Z\nPreferred-Languages: en,sk,cs",
    )
}
#[get("/robots.txt")]
async fn robots_txt() -> RawText<&'static str> {
    RawText("User-agent: *\nDisallow: /search")
}

// JavaScript templates
#[get("/static/js/hbs/<script_name>?<t>&<lang>&<loc>&<q>")]
fn script(
    script_name: &str,
    t: Option<&str>,
    lang: Option<&str>,
    loc: Option<&str>,
    q: Option<&str>,
) -> RawJavaScript<String> {
    // Create handlebars engine
    let mut hbs = Handlebars::new();

    // Read the template file
    let template_path = format!("static/js/hbs/{}", script_name);
    let template_content = std::fs::read_to_string(&template_path)
        .unwrap_or_else(|_| format!("console.error('Template {} not found');", template_path));

    hbs.register_template_string("js_template", &template_content)
        .unwrap(); // Register the template

    RawJavaScript(
        hbs.render(
            "js_template",
            &serde_json::json!({
                "q": q.unwrap_or_default(),
                "t": t.unwrap_or_default(),
                "lang": lang.unwrap_or_default(),
                "loc": loc.unwrap_or_default(),
            }),
        )
        .unwrap_or_else(|_| format!("console.error('Failed to render {}');", script_name)),
    )
}

// PriEco favicons
#[rocket::get("/static/prieco_favicons/<filename>")]
async fn favicon(filename: String) -> Result<DecompressedImage, Status> {
    // Sanitize the filename to prevent directory traversal
    if filename.contains("..") || filename.contains("/") {
        return Err(Status::BadRequest);
    }

    // The filename comes in as "name.ext", we need to look for "name.ext.br"
    let file_path = format!("static/prieco_favicons/{}.br", filename);
    let path = Path::new(&file_path);

    if !path.exists() {
        return Err(Status::NotFound);
    }

    // Rest of the decompression logic remains the same...
    let compressed_data = match std::fs::read(path) {
        Ok(data) => data,
        Err(_) => return Err(Status::InternalServerError),
    };

    let mut decompressed = Vec::new();
    let mut decoder = brotli::Decompressor::new(compressed_data.as_slice(), 4096);

    match decoder.read_to_end(&mut decompressed) {
        Ok(_) => {
            let content_type = detect_image_type(&decompressed);
            Ok(DecompressedImage {
                data: decompressed,
                content_type,
            })
        }
        Err(_) => Err(Status::InternalServerError),
    }
}

////
// Terms
////
// JavaScript templates
#[get("/privacy")]
fn privacy(cookie_jar: &CookieJar<'_>) -> Template {
    let mut context: HashMap<String, RocketValue> = HashMap::from([
        (String::from("css_version"), json!(CSS_VERSION)),
        (String::from("js_version"), json!(JS_VERSION)),
        (String::from("title_query"), json!("Privacy Policy | ")),
    ]);

    settings::run(&mut context, &None, cookie_jar);

    Template::render("legal/privacy", context)
}

////
// Landing page
////
#[get("/")]
fn index(client_ip: ClientIp, cookie_jar: &CookieJar<'_>) -> Template {
    let ip_addr = client_ip.0; // Extract IP address

    let mut context: HashMap<String, RocketValue> = HashMap::new();

    context.insert(String::from("css_version"), json!(CSS_VERSION));
    context.insert(String::from("js_version"), json!(JS_VERSION));

    settings::run(&mut context, &Some(ip_addr), cookie_jar);

    Template::render("home", &context)
}
// Health check
#[head("/")]
fn index_head() -> &'static str {
    ""
}

////
// Search
////
#[get("/search?<t>&<q>&<sxprmedia>&<sxprsearchsugg>")]
async fn search(
    t: &str,
    q: &str,
    #[allow(unused_variables)] sxprmedia: Option<&str>, // Search Expander data
    #[allow(unused_variables)] sxprsearchsugg: Option<&str>, // Search Expander data
    client_ip: ClientIp,
    cookie_jar: &CookieJar<'_>,
) -> Template {
    ////
    // Create context
    ////
    let mut context: HashMap<String, Value> = HashMap::from([
        (String::from("css_version"), json!(CSS_VERSION)),
        (String::from("js_version"), json!(JS_VERSION)),
        (String::from("title_query"), json!(format!("{} | ", q))),
        (String::from("query"), json!(q)),
        (String::from("type"), json!(t)),
        (
            String::from("lang"),
            json!(
                cookie_jar
                    .get("lang")
                    .map(|c| c.value().to_string())
                    .unwrap_or_else(|| "all".to_string())
            ),
        ),
        (
            String::from("loc"),
            json!(
                cookie_jar
                    .get("loc")
                    .map(|c| c.value().to_string())
                    .unwrap_or_else(|| "all".to_string())
            ),
        ),
        (
            String::from("placeholder_number"),
            json!(vec![json!(()); 10]),
        ),
    ]);

    // Search type
    if let Some(key) = match t {
        "all" => Some("all_active"),
        "img" => Some("img_active"),
        "vid" => Some("vid_active"),
        "new" => Some("new_active"),
        "shop" => Some("shop_active"),
        "map" => Some("map_active"),
        _ => None,
    } {
        context.insert(String::from(key), json!("type_active"));
    }

    if q.contains("!") {
        context.insert(String::from("bang"), json!(true));
    } else {
        context.insert(String::from("bang"), json!(false));
    }

    settings::run(&mut context, &Some(client_ip.0), cookie_jar); // Aplly cookies' settings to context

    Template::render("search", &context)
}
#[get("/results_html?<t>&<q>&<loc>&<lang>")]
async fn results_htmls(
    t: &str,
    q: &str,
    lang: &str,
    loc: &str,
    embedding_service: &State<EmbeddingService>,
    cookie_jar: &CookieJar<'_>,
) -> Template {
    Template::render(
        "search/results",
        search_endpoint::run(t, q, lang, loc, embedding_service, cookie_jar).await,
    )
}

// PriEco API
#[get("/api?<a>&<lang>&<loc>&<q>")]
async fn api(
    a: &str,
    lang: &str,
    loc: &str,
    q: &str,
    embedding_service: &State<EmbeddingService>,
) -> Json<Vec<RocketValue>> {
    if !["IWaebywkZHaQikH9YfznSanMS9c2H8dHvAtlDWWzKSfWOu83DdVfidb5khjn"].contains(&a) {
        return Json(vec![]);
    }

    let (_, results) = search_db::run_json(q, lang, loc, embedding_service).await;
    Json(results)
}

////
// Settings
////
#[get("/settings_html")]
fn settings_htmls(cookie_jar: &CookieJar<'_>) -> Template {
    let mut context: HashMap<String, RocketValue> = HashMap::new();

    context.insert(String::from("css_version"), json!(CSS_VERSION));
    context.insert(String::from("js_version"), json!(JS_VERSION));

    if !cookie_jar.get("index").is_some() {
        context.insert(
            String::from("prieco_user_stats"),
            json!(
                (format!(
                    "{:.2}",
                    cookie_jar
                        .get("prieco_searches")
                        .and_then(|c| c.value().parse::<u64>().ok())
                        .unwrap_or(1) as f64
                        / cookie_jar
                            .get("all_searches")
                            .and_then(|c| c.value().parse::<u64>().ok())
                            .unwrap_or(1) as f64
                        * 100.0
                ))
            ),
        );
    } else {
        context.insert(String::from("prieco_user_stats"), json!(100.0));
    }

    settings::run(&mut context, &None, cookie_jar);

    Template::render("settings", &context)
}

////
// Proxy
////
#[get("/proxy?<u>&<width>&<height>")]
pub async fn proxy_get(
    u: &str,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<(ContentType, Vec<u8>), Status> {
    proxy_request(u, None, "GET", width, height).await
}

#[post("/proxy?<u>&<width>&<height>", data = "<body>")]
pub async fn proxy_post(
    u: &str,
    body: Vec<u8>,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<(ContentType, Vec<u8>), Status> {
    proxy_request(u, Some(body), "POST", width, height).await
}

////
// Helper functions
////
// Favicons
pub struct DecompressedImage {
    data: Vec<u8>,
    content_type: ContentType,
}

impl<'r> Responder<'r, 'static> for DecompressedImage {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        Response::build()
            .header(self.content_type)
            .sized_body(self.data.len(), Cursor::new(self.data))
            .ok()
    }
}
fn detect_image_type(data: &[u8]) -> ContentType {
    if data.len() < 8 {
        return ContentType::Binary;
    }

    // Check file signatures (magic numbers)
    match &data[0..8] {
        // PNG: 89 50 4E 47 0D 0A 1A 0A
        [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] => ContentType::PNG,
        // JPEG: FF D8 FF
        [0xFF, 0xD8, 0xFF, ..] => ContentType::JPEG,
        // GIF87a: 47 49 46 38 37 61
        [0x47, 0x49, 0x46, 0x38, 0x37, 0x61, ..] => ContentType::GIF,
        // GIF89a: 47 49 46 38 39 61
        [0x47, 0x49, 0x46, 0x38, 0x39, 0x61, ..] => ContentType::GIF,
        // SVG: Check if it starts with < and contains "svg"
        [0x3C, ..] => {
            if let Ok(text) = std::str::from_utf8(&data[0..100.min(data.len())]) {
                if text.to_lowercase().contains("svg") {
                    return ContentType::SVG;
                }
            }
            ContentType::Binary
        }
        // ICO: 00 00 01 00
        [0x00, 0x00, 0x01, 0x00, ..] => ContentType::Icon,
        // WebP: RIFF....WEBP
        [0x52, 0x49, 0x46, 0x46, _, _, _, _] if data.len() >= 12 && &data[8..12] == b"WEBP" => {
            ContentType::new("image", "webp")
        }
        _ => ContentType::Binary,
    }
}

// Proxy
async fn proxy_request(
    url: &str,
    body: Option<Vec<u8>>,
    method: &str,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<(ContentType, Vec<u8>), Status> {
    let decoded_url = match urlencoding::decode(url) {
        Ok(url) => url.to_string(),
        Err(_) => return Err(Status::BadRequest),
    };

    if !is_valid_url(&decoded_url) {
        return Err(Status::BadRequest);
    }

    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.0.0 Safari/537.36")
        .gzip(true)
        .brotli(true)
        .deflate(true)
        .build()
        .unwrap();

    let request_builder = match method {
        "GET" => client
            .get(&decoded_url)
            .header("Referer", "https://dev.prieco.net/"),
        "POST" => {
            let mut req = client
                .post(&decoded_url)
                .header("Referer", "https://dev.prieco.net/");
            if let Some(body_data) = body {
                req = req
                    .body(body_data)
                    .header("Content-Type", "application/json");
            }
            req
        }
        _ => return Err(Status::MethodNotAllowed),
    };

    let resp = match request_builder.send().await {
        Ok(response) => response,
        Err(e) => {
            println!("Request failed for URL: {}", decoded_url);
            println!("Error: {:?}", e);
            return Err(Status::BadGateway);
        }
    };

    if !resp.status().is_success() {
        println!("Fetched URL: {}", decoded_url);
        println!("Status: {}", resp.status());
        if let Some(ct) = resp.headers().get("content-type") {
            println!("Content-Type: {:?}", ct);
        }
        return Err(Status::NotFound);
    }

    let content_type = if let Some(ct) = resp.headers().get("content-type") {
        if let Ok(ct_str) = ct.to_str() {
            match ct_str {
                s if s.starts_with("text/javascript") => ContentType::JavaScript,
                s if s.starts_with("application/javascript") => ContentType::JavaScript,
                s if s.starts_with("text/css") => ContentType::CSS,
                s if s.starts_with("text/html") => ContentType::HTML,
                s if s.starts_with("application/json") => ContentType::JSON,
                s if s.starts_with("image/") => {
                    if s.contains("png") {
                        ContentType::PNG
                    } else if s.contains("jpeg") || s.contains("jpg") {
                        ContentType::JPEG
                    } else if s.contains("gif") {
                        ContentType::GIF
                    } else if s.contains("webp") {
                        ContentType::WEBP
                    } else if s.contains("svg") {
                        ContentType::SVG
                    } else {
                        ContentType::Binary
                    }
                }
                _ => ContentType::Binary,
            }
        } else {
            detect_content_type_from_url(&decoded_url)
        }
    } else {
        detect_content_type_from_url(&decoded_url)
    };

    let mut body = match resp.bytes().await {
        Ok(bytes) => bytes.to_vec(),
        Err(_) => return Err(Status::InternalServerError),
    };

    if (width.is_some() || height.is_some())
        && (content_type == ContentType::PNG
            || content_type == ContentType::JPEG
            || content_type == ContentType::GIF
            || content_type == ContentType::WEBP
            || content_type == ContentType::SVG)
    {
        body = match resize_image(&body, width, height) {
            Ok(resized) => resized,
            Err(_) => return Err(Status::InternalServerError),
        };
    }

    Ok((content_type, body))
}
fn detect_content_type_from_url(url: &str) -> ContentType {
    let path = Path::new(url);
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("js") => ContentType::JavaScript,
        Some("css") => ContentType::CSS,
        Some("html") | Some("htm") => ContentType::HTML,
        Some("json") => ContentType::JSON,
        Some("png") => ContentType::PNG,
        Some("jpg") | Some("jpeg") => ContentType::JPEG,
        Some("gif") => ContentType::GIF,
        Some("webp") => ContentType::WEBP,
        Some("svg") => ContentType::SVG,
        Some("pdf") => ContentType::PDF,
        Some("txt") => ContentType::Text,
        _ => ContentType::Binary,
    }
}
fn resize_image(
    data: &[u8],
    target_width: Option<u32>,
    target_height: Option<u32>,
) -> Result<Vec<u8>, image::ImageError> {
    let img = image::load_from_memory(data)?;

    let (orig_width, orig_height) = img.dimensions();

    // Decide target size while keeping aspect ratio
    let (new_width, new_height) = match (target_width, target_height) {
        (Some(w), None) => {
            let h = (orig_height as f32 * (w as f32 / orig_width as f32)).round() as u32;
            (w, h)
        }
        (None, Some(h)) => {
            let w = (orig_width as f32 * (h as f32 / orig_height as f32)).round() as u32;
            (w, h)
        }
        (Some(w), Some(h)) => (w, h), // caller forces both, aspect ratio may break
        _ => (orig_width, orig_height),
    };

    let resized = img.resize_exact(new_width, new_height, image::imageops::Lanczos3);

    // Encode back to the same format (e.g. JPEG if original was JPEG)
    let mut out = Cursor::new(Vec::new());
    match image::guess_format(data)? {
        image::ImageFormat::Jpeg => resized.write_to(&mut out, image::ImageFormat::Jpeg)?,
        image::ImageFormat::Png => resized.write_to(&mut out, image::ImageFormat::Png)?,
        image::ImageFormat::Gif => resized.write_to(&mut out, image::ImageFormat::Gif)?,
        image::ImageFormat::WebP => resized.write_to(&mut out, image::ImageFormat::WebP)?,
        _ => resized.write_to(&mut out, image::ImageFormat::Png)?,
    }

    Ok(out.into_inner())
}
