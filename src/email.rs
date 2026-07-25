use reqwest::Client;
use serde::Serialize;
use std::env;

#[derive(Serialize)]
struct EmailRequest {
    from: String,
    to: Vec<String>,
    subject: String,
    text: String,
}

#[derive(Serialize)]
struct ResendRequest {
    #[serde(rename = "from")]
    from: String,

    #[serde(rename = "to")]
    to: Vec<String>,

    #[serde(rename = "subject")]
    subject: String,

    #[serde(rename = "text")]
    text: String,
}

pub async fn send_new_post_notification(
    username: &str,
    content: &str,
    image_url: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("RESEND_API_KEY")?;

    let body = ResendRequest {
        from: "Nozawana <onboarding@resend.dev>".to_string(),
        to: vec!["finniemcansh@gmail.com".to_string()],
        subject: "New blog post".to_string(),
        text: format!(
            "User: {}\n\n{}\n\nImage: {}",
            username,
            content,
            image_url.unwrap_or("No image")
        ),
    };

    let client = Client::new();

    client
        .post("https://api.resend.com/emails")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await?
        .error_for_status()?;

    Ok(())
}
