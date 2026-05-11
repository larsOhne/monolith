import { useState, useEffect } from 'react'

interface GraphData {
  nodes: Array<{ id: string; label: string; type?: string }>
  edges: Array<{ source: string; target: string; label?: string }>
}

interface Props { apiBase: string | null }

export default function GraphView({ apiBase }: Props) {
  const [graph, setGraph] = useState<GraphData | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [query, setQuery] = useState('')
  const [queryResult, setQueryResult] = useState<string | null>(null)

  const fetchGraph = async () => {
    if (!apiBase) return
    setLoading(true)
    try {
      const r = await fetch(`${apiBase}/api/graph`)
      if (!r.ok) throw new Error(await r.text())
      setGraph(await r.json())
      setError(null)
    } catch (e) {
      setError(String(e))
    } finally {
      setLoading(false)
    }
  }

  const buildGraph = async () => {
    if (!apiBase) return
    setLoading(true)
    try {
      const r = await fetch(`${apiBase}/api/graph/build`, { method: 'POST' })
      if (!r.ok) throw new Error(await r.text())
      await fetchGraph()
    } catch (e) {
      setError(String(e))
      setLoading(false)
    }
  }

  const runQuery = async () => {
    if (!apiBase || !query.trim()) return
    try {
      const r = await fetch(`${apiBase}/api/graph/query?q=${encodeURIComponent(query)}`)
      if (!r.ok) throw new Error(await r.text())
      const data = await r.json()
      setQueryResult(data.result ?? JSON.stringify(data))
    } catch (e) {
      setQueryResult(`Error: ${e}`)
    }
  }

  useEffect(() => { fetchGraph() }, [apiBase])

  return (
    <div style={{ padding: '24px 32px', overflowY: 'auto', height: '100%' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 20 }}>
        <h1 style={{ fontSize: 20, fontWeight: 600, flex: 1 }}>Knowledge Graph</h1>
        <button onClick={fetchGraph} disabled={loading || !apiBase}>↻ Refresh</button>
        <button className="primary" onClick={buildGraph} disabled={loading || !apiBase}>Build Graph</button>
      </div>

      {!apiBase && <p style={{ color: 'var(--muted)' }}>Waiting for backend…</p>}
      {error && <p style={{ color: 'var(--danger)', marginBottom: 12 }}>{error}</p>}
      {loading && <p style={{ color: 'var(--muted)' }}>Loading…</p>}

      {/* Query */}
      {graph && (
        <div style={{ marginBottom: 24 }}>
          <div style={{ display: 'flex', gap: 8, marginBottom: 8 }}>
            <input
              value={query}
              onChange={e => setQuery(e.target.value)}
              placeholder="Query the graph…"
              onKeyDown={e => e.key === 'Enter' && runQuery()}
              style={{ flex: 1 }}
            />
            <button onClick={runQuery}>Ask</button>
          </div>
          {queryResult && (
            <pre style={{ fontSize: 12, whiteSpace: 'pre-wrap', wordBreak: 'break-word' }}>
              {queryResult}
            </pre>
          )}
        </div>
      )}

      {/* Nodes/Edges summary */}
      {graph && (
        <div style={{ display: 'flex', gap: 16, marginBottom: 16 }}>
          <div style={{ background: 'var(--surface)', border: '1px solid var(--border)', borderRadius: 'var(--radius)', padding: '12px 20px', textAlign: 'center' }}>
            <div style={{ fontSize: 24, fontWeight: 600, color: 'var(--accent)' }}>{graph.nodes.length}</div>
            <div style={{ fontSize: 12, color: 'var(--muted)' }}>Nodes</div>
          </div>
          <div style={{ background: 'var(--surface)', border: '1px solid var(--border)', borderRadius: 'var(--radius)', padding: '12px 20px', textAlign: 'center' }}>
            <div style={{ fontSize: 24, fontWeight: 600, color: 'var(--accent)' }}>{graph.edges.length}</div>
            <div style={{ fontSize: 12, color: 'var(--muted)' }}>Edges</div>
          </div>
        </div>
      )}

      {graph && graph.nodes.length > 0 && (
        <div>
          <div style={{ fontSize: 13, fontWeight: 500, marginBottom: 8, color: 'var(--muted)' }}>Nodes</div>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8 }}>
            {graph.nodes.map(n => (
              <div key={n.id} style={{
                background: 'var(--surface)',
                border: '1px solid var(--border)',
                borderRadius: 20,
                padding: '3px 12px',
                fontSize: 12,
                color: 'var(--fg)',
              }}>
                {n.label || n.id}
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  )
}
