const BASE = '/api'

export async function apiFetch<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    headers: { 'Content-Type': 'application/json', ...init?.headers },
    ...init,
  })
  if (!res.ok) {
    const err = await res.text()
    throw new Error(`${res.status} ${err}`)
  }
  return res.json()
}

export const api = {
  projects: {
    list: () => apiFetch<Project[]>('/projects'),
    create: (body: { name: string; slug: string; description?: string }) =>
      apiFetch<Project>('/projects', { method: 'POST', body: JSON.stringify(body) }),
  },
  sources: {
    list: (slug: string) => apiFetch<Source[]>(`/sources?project_slug=${slug}`),
    add: (body: { project_slug: string; file_path?: string; url?: string }) =>
      apiFetch<Source>('/sources', { method: 'POST', body: JSON.stringify(body) }),
    content: (id: string) => apiFetch<{ content: string }>(`/sources/${id}/content`),
  },
  evidence: {
    pin: (body: { source_id: string; verbatim_text: string }) =>
      apiFetch<Evidence>('/evidence', { method: 'POST', body: JSON.stringify(body) }),
    search: (query: string) => apiFetch<Evidence[]>(`/evidence/search?query=${encodeURIComponent(query)}`),
  },
  statements: {
    list: (slug: string) => apiFetch<Statement[]>(`/statements?project_slug=${slug}`),
    create: (body: { project_slug: string; content: string; evidence_ids: string[] }) =>
      apiFetch<Statement>('/statements', { method: 'POST', body: JSON.stringify(body) }),
    provenance: (id: string) => apiFetch<ProvenanceChain>(`/statements/${id}/provenance`),
  },
  graph: {
    get: (slug: string) => apiFetch<object>(`/graph/${slug}`),
    build: (slug: string) => apiFetch<{ graph_json: string }>(`/graph/${slug}/build`, { method: 'POST' }),
    query: (slug: string, q: string) =>
      apiFetch<{ result: string }>(`/graph/${slug}/query?q=${encodeURIComponent(q)}`),
  },
  drift: {
    check: (slug: string) => apiFetch<DriftReport>(`/drift/${slug}`),
  },
}

// Shared types
export interface Project { id: string; name: string; slug: string; description: string }
export interface Source { id: string; project_id: string; path: string; url?: string; sha256: string; git_sha: string; ingested_at: string }
export interface Evidence { id: string; source_id: string; verbatim_text: string; char_start: number; char_end: number; git_sha_at_pin: string; status: string }
export interface Statement { id: string; project_id: string; content: string; evidence_ids: string[]; created_at: string }
export interface ProvenanceChain { statement: object; evidence: object[] }
export interface DriftReport { has_issues: boolean; entries: DriftEntry[] }
export interface DriftEntry { evidence_id: string; source_id: string; old_git_sha: string; new_git_sha: string; status: string; diff_snippet: string }
