use crate::suwayomi::types::{
    ChapterMeta, ExtensionInfo, MangaDetail, SearchPage, SearchResult, SourceInfo,
};
use regex::Regex;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::path::Path;
use std::sync::LazyLock;
use thasia_core::{Result, ThasiaError};
use tracing::warn;

#[derive(Clone)]
pub struct SuwayomiClient {
    base_url: String,
    graphql_url: String,
    http: reqwest::Client,
}

impl SuwayomiClient {
    pub fn new(port: u16) -> Self {
        Self {
            base_url: format!("http://127.0.0.1:{port}/api/v1"),
            graphql_url: format!("http://127.0.0.1:{port}/api/graphql"),
            http: reqwest::Client::new(),
        }
    }

    pub async fn list_extensions(&self) -> Result<Vec<ExtensionInfo>> {
        let extensions: Vec<ExtensionRest> = self.rest_get("extension/list").await?;
        Ok(extensions
            .into_iter()
            .map(|e| ExtensionInfo {
                pkg_name: e.package_name,
                name: e.name,
                lang: e.lang,
                version_name: e.version_name,
                installed: e.installed,
            })
            .collect())
    }

    pub async fn install_extension(&self, pkg: &str) -> Result<()> {
        self.rest_status(&format!("extension/install/{pkg}")).await?;
        Ok(())
    }

    pub async fn uninstall_extension(&self, pkg: &str) -> Result<()> {
        self.rest_status(&format!("extension/uninstall/{pkg}")).await?;
        Ok(())
    }

    pub async fn list_sources(&self) -> Result<Vec<SourceInfo>> {
        let query = r#"
            query GetSources {
                sources {
                    nodes {
                        id
                        name
                        lang
                    }
                }
            }
        "#;
        let resp: SourcesResponse = self.query(query, None).await?;
        Ok(resp
            .sources
            .nodes
            .into_iter()
            .map(|s| SourceInfo {
                id: s.id,
                name: s.name,
                lang: s.lang,
            })
            .collect())
    }

    pub async fn search(&self, source_id: &str, q: &str, page: u32) -> Result<SearchPage> {
        // Suwayomi v2.x exposes source catalog search as a mutation, not a nested
        // query field. `type: SEARCH` fetches results matching the given query;
        // `POPULAR` / `LATEST` are the other variants for browse pages.
        let mutation = r#"
            mutation FetchSourceManga($source: LongString!, $query: String, $page: Int!) {
                fetchSourceManga(input: {
                    source: $source
                    query: $query
                    page: $page
                    type: SEARCH
                }) {
                    hasNextPage
                    mangas {
                        id
                        title
                        thumbnailUrl
                    }
                }
            }
        "#;
        let vars = serde_json::json!({
            "source": source_id,
            "query": q,
            "page": page
        });
        let resp: SearchResponse = self.query(mutation, Some(vars)).await?;
        let payload = resp.fetch_source_manga;
        Ok(SearchPage {
            results: payload
                .mangas
                .into_iter()
                .map(|item| SearchResult {
                    id: item.id,
                    title: item.title,
                    thumbnail_url: self.absolute_url(item.thumbnail_url),
                    initialized: true,
                })
                .collect(),
            has_next_page: payload.has_next_page,
        })
    }

    pub async fn manga(&self, manga_id: i64) -> Result<MangaDetail> {
        let query = r#"
            query GetManga($id: Int!) {
                manga(id: $id) {
                    id
                    title
                    author
                    artist
                    description
                    thumbnailUrl
                }
            }
        "#;
        let vars = serde_json::json!({ "id": manga_id });
        let resp: MangaResponse = self.query(query, Some(vars)).await?;
        let m = resp.manga;
        Ok(MangaDetail {
            id: m.id,
            title: m.title,
            author: m.author,
            artist: m.artist,
            description: m.description,
            thumbnail_url: self.absolute_url(m.thumbnail_url),
        })
    }

    pub async fn chapters(&self, manga_id: i64) -> Result<Vec<ChapterMeta>> {
        let mutation = r#"
            mutation FetchChapters($id: Int!) {
                fetchChapters(mangaId: $id) {
                    success
                }
            }
        "#;
        let vars = serde_json::json!({ "id": manga_id });
        // fetchChapters refreshes the chapter list from the remote source.
        // Failure is non-fatal: we fall back to whatever Suwayomi has cached.
        if let Err(err) = self
            .query::<serde_json::Value>(mutation, Some(vars.clone()))
            .await
        {
            warn!("fetchChapters mutation failed (using cached list): {err}");
        }

        let query = r#"
            query GetChapters($id: Int!) {
                manga(id: $id) {
                    chapters {
                        nodes {
                            id
                            name
                            chapterNumber
                            scanlator
                            isDownloaded
                        }
                    }
                }
            }
        "#;
        let resp: ChaptersResponse = self.query(query, Some(vars)).await?;
        Ok(resp
            .manga
            .chapters
            .nodes
            .into_iter()
            .map(|c| {
                let name = c.name;
                ChapterMeta {
                    id: c.id,
                    name: name.clone(),
                    chapter_number: c.chapter_number,
                    volume_number: parse_volume_from_name(&name),
                    scanlator: c.scanlator,
                    downloaded: c.is_downloaded,
                }
            })
            .collect())
    }

    pub async fn download_chapter_cbz(&self, chapter_id: i64, destination: &Path) -> Result<()> {
        let response = self
            .http
            .get(format!(
                "{}/chapter/{}/download?markAsRead=false",
                self.base_url, chapter_id
            ))
            .send()
            .await
            .map_err(|e| ThasiaError::Discovery(e.to_string()))?;
        let bytes = error_for_status_with_body(response)
            .await?
            .bytes()
            .await
            .map_err(|e| ThasiaError::Discovery(e.to_string()))?;

        if let Some(parent) = destination.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(ThasiaError::Io)?;
        }
        tokio::fs::write(destination, bytes)
            .await
            .map_err(ThasiaError::Io)
    }

    async fn query<T: DeserializeOwned>(
        &self,
        query: &str,
        variables: Option<serde_json::Value>,
    ) -> Result<T> {
        let body = serde_json::json!({
            "query": query,
            "variables": variables.unwrap_or(serde_json::json!({}))
        });

        let response = self
            .http
            .post(&self.graphql_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ThasiaError::Discovery(e.to_string()))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| ThasiaError::Discovery(e.to_string()))?;

        if !status.is_success() {
            return Err(ThasiaError::Discovery(format!(
                "GraphQL error ({}): {}",
                status, text
            )));
        }

        let gql_resp: GqlResponse<T> =
            serde_json::from_str(&text).map_err(|e| ThasiaError::Discovery(e.to_string()))?;

        if let Some(errors) = gql_resp.errors
            && !errors.is_empty()
        {
            return Err(ThasiaError::Discovery(format!(
                "GraphQL errors: {:?}",
                errors
            )));
        }

        gql_resp.data.ok_or_else(|| {
            ThasiaError::Discovery("GraphQL response contained no data and no errors".into())
        })
    }

    async fn rest_get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let text = self.rest_status(path).await?;
        serde_json::from_str(&text).map_err(|e| ThasiaError::Discovery(e.to_string()))
    }

    async fn rest_status(&self, path: &str) -> Result<String> {
        let url = format!("{}/{}", self.base_url, path.trim_start_matches('/'));
        let response = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|e| ThasiaError::Discovery(e.to_string()))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| ThasiaError::Discovery(e.to_string()))?;

        if !status.is_success() && !status.is_redirection() {
            return Err(ThasiaError::Discovery(format!(
                "Suwayomi REST error ({}): {}",
                status, text
            )));
        }

        Ok(text)
    }

    fn absolute_url(&self, url: Option<String>) -> Option<String> {
        let url = url?;
        if url.starts_with("http://") || url.starts_with("https://") {
            return Some(url);
        }
        if url.starts_with('/') {
            let origin = self.base_url.trim_end_matches("/api/v1");
            return Some(format!("{origin}{url}"));
        }
        Some(url)
    }
}

#[derive(Deserialize)]
struct GqlResponse<T> {
    data: Option<T>,
    errors: Option<Vec<serde_json::Value>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExtensionRest {
    #[serde(rename = "pkgName", alias = "pkg")]
    package_name: String,
    name: String,
    lang: Option<String>,
    #[serde(alias = "version")]
    version_name: Option<String>,
    #[serde(rename = "isInstalled", alias = "installed", default)]
    installed: bool,
}

#[derive(Deserialize)]
struct SourcesResponse {
    sources: SourceNodeListGql,
}

#[derive(Deserialize)]
struct SourceNodeListGql {
    nodes: Vec<SourceGql>,
}

#[derive(Deserialize)]
struct SourceGql {
    id: String,
    name: String,
    lang: Option<String>,
}

// `fetchSourceManga` mutation payload — top-level key in the GraphQL `data` object.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchResponse {
    fetch_source_manga: FetchSourceMangaPayload,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FetchSourceMangaPayload {
    has_next_page: bool,
    mangas: Vec<MangaResultGql>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MangaResultGql {
    id: i64,
    title: String,
    thumbnail_url: Option<String>,
}

#[derive(Deserialize)]
struct MangaResponse {
    manga: MangaGql,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MangaGql {
    id: i64,
    title: String,
    author: Option<String>,
    artist: Option<String>,
    description: Option<String>,
    thumbnail_url: Option<String>,
}

#[derive(Deserialize)]
struct ChaptersResponse {
    manga: MangaChaptersGql,
}

#[derive(Deserialize)]
struct MangaChaptersGql {
    chapters: ChapterNodesGql,
}

#[derive(Deserialize)]
struct ChapterNodesGql {
    nodes: Vec<ChapterGql>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChapterGql {
    id: i64,
    name: String,
    chapter_number: f32,
    scanlator: Option<String>,
    is_downloaded: bool,
}

static VOLUME_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bvol(?:ume)?\.?\s*(\d+)\b").expect("invalid volume regex"));

fn parse_volume_from_name(name: &str) -> Option<u32> {
    VOLUME_RE
        .captures(name)
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().parse::<u32>().ok())
}

async fn error_for_status_with_body(response: reqwest::Response) -> Result<reqwest::Response> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    let url = response.url().clone();
    let body = response.text().await.unwrap_or_default();
    let message = if body.trim().is_empty() {
        format!("HTTP status {status} for url ({url})")
    } else {
        format!("HTTP status {status} for url ({url}): {}", body.trim())
    };
    Err(ThasiaError::Discovery(message))
}

#[cfg(test)]
mod tests {
    use super::parse_volume_from_name;

    #[test]
    fn parse_volume_from_name_cases() {
        let cases = [
            ("Vol.1 Ch.1 - Raphtalia and Boss", Some(1)),
            (
                "Vol.2 Ch.11 - The Hero's Slave is an Angel of Liberty",
                Some(2),
            ),
            ("Volume 3 Chapter 5", Some(3)),
            ("Vol. 4", Some(4)),
            ("vol3", Some(3)),
            ("Chapter 5", None),
            ("Oneshot", None),
        ];

        for (name, expected) in cases {
            assert_eq!(parse_volume_from_name(name), expected, "{name}");
        }
    }
}
