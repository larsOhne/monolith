import { useState } from 'react'

interface DriftEntry {
  evidence_id: string
  source_id: string
  old_git_sha: string
  new_git_sha: string
  status: string
  diff_snippet: string
}

interface DriftReport {
  has_issues: boolean
  entries: DriftEntry[]
}

const STATUS_COLOR: Record<string, string> = {
  drifted: 'var(--orange)',
  broken:  'var(--danger)',
  valid:   'var(--green)',
}

interface Props { apiBase: string | null }

export default function DriftInspector({ apiBase }: Props) {
  const [report, setReport] = useState<DriftReport | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [expanded, setExpanded] = useState<string | null>(null)

  const runDrift = async () => {
    if (!apiBase) return
    setLoading(true)
    setError(null)
    try {
      const r = await fetch(`${apiBase}/api/drift`)
      if (!r.ok) throw new Error(await r.text())
      setReport(await r.json())
    } catch (e) {
      setError(String(e))
    } finally {
      setLoading(false)
    }
  }

  return (
    <div style={{ padding: '24px 32px', overflowY: 'auto', height: '100%' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 20 }}>
        <h1 style={{ fontSize: 20, fontWeight: 600, flex: 1 }}>Drift Inspector</h1>
        <button className="primary" onClick={runDrift} disabled={loading || !apiBase}>
          {loading ? 'Checking…' : 'Check Drift'}
        </button>
      </div>

      {!apiBase && <p style={{ color: 'var(--muted)' }}>Waiting for backend…</p>}
      {error && <p style={{ color: 'var(--danger)', marginBottom: 12 }}>{error}</p>}

      {report && (
        report.has_issues ? (
          <div>
            <p style={{ marginBottom: 16, color: 'var(--orange)' }}>
              ⚡ {report.entries.length} evidence item(s) with issues
            </p>
            {report.entries.map(e => (
              <div key={e.evidence_id} style={{
                background: 'var(--surface)',
                border: '1px solid var(--border)',
                borderLeft: `4px solid ${STATUS_COLOR[e.status] ?? 'var(--border)'}`,
                borderRadius: 'var(--radius)',
                padding: '12px 16px',
                marginBottom: 10,
              }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 4 }}>
                  <span style={{ fontWeight: 600, color: STATUS_COLOR[e.status], fontSize: 12 }}>
                    {e.status.toUpperCase()}
                  </span>
                  <code style={{ fontSize: 11, color: 'var(--muted)' }}>{e.evidence_id.slice(0, 8)}…</code>
                  <button
                    style={{ marginLeft: 'auto', padding: '2px 8px', fontSize: 11 }}
                    onClick={() => setExpanded(expanded === e.evidence_id ? null : e.evidence_id)}
                  >
                    {expanded === e.evidence_id ? 'Hide diff' : 'Show diff'}
                  </button>
                </div>
                <div style={{ fontSize: 11, color: 'var(--muted)' }}>
                  {e.old_git_sha.slice(0, 8)} → {e.new_git_sha.slice(0, 8)}
                </div>
                {expanded === e.evidence_id && (
                  <pre style={{ marginTop: 8, fontSize: 11, whiteSpace: 'pre-wrap', wordBreak: 'break-word' }}>
                    {e.diff_snippet || '(no diff)'}
                  </pre>
                )}
              </div>
            ))}
          </div>
        ) : (
          <p style={{ color: 'var(--green)' }}>✓ All evidence is valid — no drift detected.</p>
        )
      )}

      {!report && !loading && (
        <p style={{ color: 'var(--muted)' }}>Run a drift check to inspect evidence freshness.</p>
      )}
    </div>
  )
}
