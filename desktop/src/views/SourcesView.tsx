import { useState, useEffect } from 'react'
import { pickFile } from '../tauri_api'

interface Source {
  id: string
  path: string
  sha256: string
  ingested_at: string
}

interface Props { apiBase: string | null }

export default function SourcesView({ apiBase }: Props) {
  const [sources, setSources] = useState<Source[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [adding, setAdding] = useState(false)

  const fetchSources = async () => {
    if (!apiBase) return
    setLoading(true)
    try {
      const r = await fetch(`${apiBase}/api/sources`)
      if (!r.ok) throw new Error(await r.text())
      setSources(await r.json())
      setError(null)
    } catch (e) {
      setError(String(e))
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => { fetchSources() }, [apiBase])

  const handleAddFile = async () => {
    const path = await pickFile([
      { name: 'Documents', extensions: ['pdf', 'md', 'txt', 'html'] },
    ])
    if (!path || !apiBase) return
    setAdding(true)
    try {
      const r = await fetch(`${apiBase}/api/sources`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ file_path: path }),
      })
      if (!r.ok) throw new Error(await r.text())
      await fetchSources()
    } catch (e) {
      setError(String(e))
    } finally {
      setAdding(false)
    }
  }

  return (
    <div style={{ padding: '24px 32px', overflowY: 'auto', height: '100%' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 20 }}>
        <h1 style={{ fontSize: 20, fontWeight: 600, flex: 1 }}>Sources</h1>
        <button onClick={fetchSources} disabled={loading || !apiBase}>↻ Refresh</button>
        <button className="primary" onClick={handleAddFile} disabled={adding || !apiBase}>
          {adding ? 'Ingesting…' : '+ Add File'}
        </button>
      </div>

      {!apiBase && (
        <p style={{ color: 'var(--muted)' }}>Waiting for backend…</p>
      )}
      {error && <p style={{ color: 'var(--danger)', marginBottom: 12 }}>{error}</p>}
      {loading && <p style={{ color: 'var(--muted)' }}>Loading…</p>}

      {sources.length === 0 && !loading && apiBase && (
        <p style={{ color: 'var(--muted)' }}>No sources ingested yet. Add a file to get started.</p>
      )}

      {sources.map(s => (
        <div key={s.id} style={{
          background: 'var(--surface)',
          border: '1px solid var(--border)',
          borderRadius: 'var(--radius)',
          padding: '12px 16px',
          marginBottom: 10,
        }}>
          <div style={{ fontFamily: 'var(--font-mono)', fontSize: 13, marginBottom: 4 }}>{s.path}</div>
          <div style={{ fontSize: 11, color: 'var(--muted)', display: 'flex', gap: 16 }}>
            <span>SHA: {s.sha256.slice(0, 12)}…</span>
            <span>Ingested: {new Date(s.ingested_at).toLocaleString()}</span>
          </div>
        </div>
      ))}
    </div>
  )
}
