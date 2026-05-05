import { useState } from 'react'
import { api, type DriftReport } from '../api'

const STATUS_COLOR: Record<string, string> = {
  drifted: 'orange',
  broken: 'red',
  valid: 'green',
}

export default function DriftInspector() {
  const [slug, setSlug] = useState('')
  const [report, setReport] = useState<DriftReport | null>(null)
  const [loading, setLoading] = useState(false)
  const [expanded, setExpanded] = useState<string | null>(null)

  async function runDrift() {
    setLoading(true)
    try {
      setReport(await api.drift.check(slug))
    } catch (err) {
      console.error(err)
    } finally {
      setLoading(false)
    }
  }

  return (
    <div>
      <h1>Drift Inspector</h1>
      <div style={{ display: 'flex', gap: '0.5rem', marginBottom: '1.5rem' }}>
        <input placeholder="project-slug" value={slug} onChange={e => setSlug(e.target.value)} />
        <button onClick={runDrift} disabled={loading}>{loading ? 'Checking…' : 'Check drift'}</button>
      </div>
      {report && (
        report.has_issues ? (
          <div>
            <p><strong>{report.entries.length}</strong> evidence item(s) with issues.</p>
            {report.entries.map(e => (
              <div key={e.evidence_id} style={{ borderLeft: `4px solid ${STATUS_COLOR[e.status] ?? '#ccc'}`, paddingLeft: '0.75rem', marginBottom: '1rem' }}>
                <div style={{ display: 'flex', gap: '0.5rem', alignItems: 'center' }}>
                  <span style={{ fontWeight: 600, color: STATUS_COLOR[e.status] }}>{e.status.toUpperCase()}</span>
                  <code style={{ fontSize: 12 }}>{e.evidence_id.slice(0, 8)}…</code>
                  <button style={{ fontSize: 12 }} onClick={() => setExpanded(expanded === e.evidence_id ? null : e.evidence_id)}>
                    {expanded === e.evidence_id ? 'Hide diff' : 'Show diff'}
                  </button>
                </div>
                <div style={{ fontSize: 12, color: '#666' }}>
                  {e.old_git_sha.slice(0, 8)} → {e.new_git_sha.slice(0, 8)}
                </div>
                {expanded === e.evidence_id && (
                  <pre style={{ background: '#f9f9f9', padding: '0.5rem', borderRadius: 4, fontSize: 12, overflowX: 'auto', marginTop: '0.5rem' }}>
                    {e.diff_snippet || '(no diff)'}
                  </pre>
                )}
              </div>
            ))}
          </div>
        ) : (
          <p style={{ color: 'green' }}>All evidence is valid — no drift detected.</p>
        )
      )}
    </div>
  )
}
