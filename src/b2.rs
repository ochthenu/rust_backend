use axum::{extract::Multipart, http::StatusCode};

use base64::{engine::general_purpose, Engine as _};

use reqwest::{
    header::{AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE},
    Client,
};

use serde::{Deserialize, Serialize};

use sha1::{Digest, Sha1};

use std::env;

use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct AuthorizeResponse {
    #[allow(dead_code)]
    #[serde(rename = "accountId")]
    account_id: String,

    #[serde(rename = "authorizationToken")]
    authorization_token: String,

    #[serde(rename = "apiInfo")]
    api_info: ApiInfo,
}

#[derive(Debug, Deserialize)]
struct ApiInfo {
    #[serde(rename = "storageApi")]
    storage_api: StorageApi,
}

#[derive(Debug, Deserialize)]
struct StorageApi {
    #[serde(rename = "apiUrl")]
    api_url: String,

    #[serde(rename = "downloadUrl")]
    download_url: String,
}

#[derive(Debug, Serialize)]
struct UploadUrlRequest<'a> {
    #[serde(rename = "bucketId")]
    bucket_id: &'a str,
}

#[derive(Debug, Deserialize)]
struct UploadUrlResponse {
    #[serde(rename = "uploadUrl")]
    upload_url: String,

    #[serde(rename = "authorizationToken")]
    authorization_token: String,
}

#[derive(Debug, Deserialize)]
struct UploadResponse {
    #[serde(rename = "fileName")]
    file_name: String,
}

struct B2Client {
    client: Client,
}

#[derive(Debug, Serialize)]
pub struct UploadResult {
    pub url: String,
}

impl B2Client {
    fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    async fn authorize(&self) -> Result<AuthorizeResponse, StatusCode> {
        let key_id = env::var("B2_KEY_ID").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let application_key =
            env::var("B2_APPLICATION_KEY").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let credentials = format!("{key_id}:{application_key}");

        let auth = format!("Basic {}", general_purpose::STANDARD.encode(credentials),);

        let response = self
            .client
            .get("https://api.backblazeb2.com/b2api/v4/b2_authorize_account")
            .header(AUTHORIZATION, auth)
            .send()
            .await
            .map_err(|_| StatusCode::BAD_GATEWAY)?;

        if !response.status().is_success() {
            eprintln!("B2 authorize failed: {}", response.status());
            return Err(StatusCode::BAD_GATEWAY);
        }

        response
            .json::<AuthorizeResponse>()
            .await
            .map_err(|_| StatusCode::BAD_GATEWAY)
    }

    async fn get_upload_url(
        &self,
        auth: &AuthorizeResponse,
    ) -> Result<UploadUrlResponse, StatusCode> {
        let bucket_id = env::var("B2_BUCKET_ID").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let request = UploadUrlRequest {
            bucket_id: &bucket_id,
        };

        let response = self
            .client
            .post(format!(
                "{}/b2api/v4/b2_get_upload_url",
                auth.api_info.storage_api.api_url
            ))
            .header(AUTHORIZATION, &auth.authorization_token)
            .json(&request)
            .send()
            .await
            .map_err(|_| StatusCode::BAD_GATEWAY)?;

        if !response.status().is_success() {
            eprintln!("B2 get_upload_url failed: {}", response.status());

            return Err(StatusCode::BAD_GATEWAY);
        }

        response
            .json::<UploadUrlResponse>()
            .await
            .map_err(|_| StatusCode::BAD_GATEWAY)
    }

    fn sha1_hex(data: &[u8]) -> String {
        let mut hasher = Sha1::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    fn new_filename() -> String {
        format!("{}.jpg", Uuid::new_v4())
    }

    async fn upload_file(
        &self,
        auth: &AuthorizeResponse,
        upload: &UploadUrlResponse,
        filename: &str,
        bytes: Vec<u8>,
    ) -> Result<String, StatusCode> {
        let sha1 = Self::sha1_hex(&bytes);

        let response = self
            .client
            .post(&upload.upload_url)
            .header(AUTHORIZATION, &upload.authorization_token)
            .header("X-Bz-File-Name", filename)
            .header(CONTENT_TYPE, "b2/x-auto")
            .header("X-Bz-Content-Sha1", sha1)
            .header(CONTENT_LENGTH, bytes.len())
            .body(bytes)
            .send()
            .await
            .map_err(|_| StatusCode::BAD_GATEWAY)?;

        if !response.status().is_success() {
            eprintln!("B2 upload failed: {}", response.status());

            return Err(StatusCode::BAD_GATEWAY);
        }

        let uploaded = response
            .json::<UploadResponse>()
            .await
            .map_err(|_| StatusCode::BAD_GATEWAY)?;

        let bucket = env::var("B2_BUCKET_NAME").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(format!(
            "{}/file/{}/{}",
            auth.api_info.storage_api.download_url, bucket, uploaded.file_name
        ))
    }
}

pub async fn upload_image(
    mut multipart: Multipart,
) -> Result<axum::Json<UploadResult>, StatusCode> {
    let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
    else {
        return Err(StatusCode::BAD_REQUEST);
    };

    let bytes = field
        .bytes()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .to_vec();

    let client = B2Client::new();

    let auth = client.authorize().await?;

    let upload = client.get_upload_url(&auth).await?;

    let filename = B2Client::new_filename();

    let url = client.upload_file(&auth, &upload, &filename, bytes).await?;

    Ok(axum::Json(UploadResult { url }))
}
