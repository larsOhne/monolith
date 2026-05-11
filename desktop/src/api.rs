/// Typed HTTP client wrappers around the monolith REST API.
/// All calls are synchronous (reqwest blocking) so they can be called from
/// egui's immediate-mode update loop via `std::thread::spawn` tasks that write
/// results back through `std::sync::mpsc` channels.
use serde::{Deserialize, Serialize};

// ─── Domain types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Source {
    pub id: String,
    pub project_id: String,
    pub path: String,
    pub sha256: String,
    pub git_sha: String,
    pub ingested_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Evidence {
    pub id: String,
    pub source_id: String,
    pub verbatim_text: String,
    pub char_start: usize,
    pub char_end: usize,
    pub git_sha_at_pin: String,
    pub status: String, // "valid" | "drifted" | "broken"
}

#[derive(Debug, Clone, Deserialize)]
pub struct Statement {
    pub id: String,
    pub content: String,
    pub evidence_ids: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DriftEntry {
    pub evidence_id: String,
    pub source_id: String,
    pub status: String,
    pub diff: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DriftReport {
    pub project_slug: String,
    pub entries: Vec<DriftEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProvenanceChain {
    pub statement: Statement,
    pub evidence: Vec<Evidence>,
    pub sources: Vec<Source>,
}

// ─── Request bodies ─────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct CreateProjectBody<'a> {
    pub name: &'a str,
    pub slug: &'a str,
    pub description: &'a str,
}

#[derive(Serialize)]
pub struct PinEvidenceBody<'a> {
    pub source_id: &'a str,
    pub verbatim_text: &'a str,
}

#[derive(Serialize)]
pub struct CreateStatementBody<'a> {
    pub project_slug: &'a str,
    pub content: &'a str,
    pub evidence_ids: &'a [String],
}

// ─── Client ─────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct Client {
    base: String,
    inner: reqwest::blocking::Client,
}

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("HTTP error {0}: {1}")]
    Http(u16, String),
    #[error("Request failed: {0}")]
    Reqwest(#[from] reqwest::Error),
}

impl Client {
    pub fn new(base_url: &str) -> Self {
        Self {
            base: base_url.trim_end_matches('/').to_string(),
            inner: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("reqwest client"),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }

    fn check<T: for<'de> serde::Deserialize<'de>>(
        resp: reqwest::blocking::Response,
    ) -> ApiResult<T> {
        let status = resp.status();
        if status.is_success() {
            Ok(resp.json::<T>()?)
        } else {
            let msg = resp.text().unwrap_or_default();
            Err(ApiError::Http(status.as_u16(), msg))
        }
    }

    // ── Projects ────────────────────────────────────────────────────────────

    pub fn list_projects(&self) -> ApiResult<Vec<Project>> {
        Self::check(self.inner.get(self.url("/api/projects")).send()?)
    }

    pub fn create_project(&self, name: &str, slug: &str, description: &str) -> ApiResult<Project> {
        Self::check(
            self.inner
                .post(self.url("/api/projects"))
                .json(&CreateProjectBody { name, slug, description })
                .send()?,
        )
    }

    // ── Sources ─────────────────────────────────────────────────────────────

    pub fn list_sources(&self, project_slug: &str) -> ApiResult<Vec<Source>> {
        Self::check(
            self.inner
                .get(self.url("/api/sources"))
                .query(&[("project_slug", project_slug)])
                .send()?,
        )
    }

    pub fn ingest_source(&self, project_slug: &str, path: &str) -> ApiResult<Source> {
        use std::collections::HashMap;
        let mut body = HashMap::new();
        body.insert("project_slug", project_slug);
        body.insert("file_path", path);
        Self::check(self.inner.post(self.url("/api/sources")).json(&body).send()?)
    }

    pub fn ingest_source_url(&self, project_slug: &str, url: &str) -> ApiResult<Source> {
        use std::collections::HashMap;
        let _ = project_slug; // vault-first: no project scope on new API
        let mut body = HashMap::new();
        body.insert("url", url);
        Self::check(self.inner.post(self.url("/api/sources")).json(&body).send()?)
    }

    pub fn source_content(&self, source_id: &str) -> ApiResult<String> {
        let resp = self.inner.get(self.url(&format!("/api/sources/{source_id}/content"))).send()?;
        let status = resp.status();
        if status.is_success() {
            Ok(resp.text()?)
        } else {
            Err(ApiError::Http(status.as_u16(), resp.text().unwrap_or_default()))
        }
    }

    // ── Evidence ────────────────────────────────────────────────────────────

    pub fn pin_evidence(&self, source_id: &str, verbatim_text: &str) -> ApiResult<Evidence> {
        Self::check(
            self.inner
                .post(self.url("/api/evidence"))
                .json(&PinEvidenceBody { source_id, verbatim_text })
                .send()?,
        )
    }

    pub fn search_evidence(&self, query: &str) -> ApiResult<Vec<Evidence>> {
        Self::check(
            self.inner
                .get(self.url("/api/evidence/search"))
                .query(&[("query", query)])
                .send()?,
        )
    }

    // ── Statements ──────────────────────────────────────────────────────────

    pub fn list_statements(&self, project_slug: &str) -> ApiResult<Vec<Statement>> {
        Self::check(
            self.inner
                .get(self.url("/api/statements"))
                .query(&[("project_slug", project_slug)])
                .send()?,
        )
    }

    pub fn create_statement(
        &self,
        project_slug: &str,
        content: &str,
        evidence_ids: &[String],
    ) -> ApiResult<Statement> {
        Self::check(
            self.inner
                .post(self.url("/api/statements"))
                .json(&CreateStatementBody { project_slug, content, evidence_ids })
                .send()?,
        )
    }

    pub fn get_provenance(&self, statement_id: &str) -> ApiResult<ProvenanceChain> {
        Self::check(
            self.inner
                .get(self.url(&format!("/api/statements/{statement_id}/provenance")))
                .send()?,
        )
    }

    // ── Drift ───────────────────────────────────────────────────────────────

    pub fn check_drift(&self, project_slug: &str) -> ApiResult<DriftReport> {
        Self::check(
            self.inner
                .get(self.url(&format!("/api/drift/{project_slug}")))
                .send()?,
        )
    }
}
