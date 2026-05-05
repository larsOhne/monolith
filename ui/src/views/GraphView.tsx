import { useState } from 'react'
import { api } from '../api'

export default function GraphView() {
  const [slug, setSlug] = useState('')
  const [query, setQuery] = useState('')
  const [queryResult, setQueryResult] = useState('')
  const [building, setBuilding] = useState(false)
  const [msg, setMsg] = useState('')

  async function buildGraph() {
    setBuilding(true); setMsg('')
    try {
      const r = await api.graph.build(slug)
      setMsg(`Graph built: ${(r as any).graph_json}`)
    } catch (err) {
      setMsg(String(err))
    } finally {
      setBuilding(false)
    }
  }

  async function runQuery(e: React.FormEvent) {
    e.preventDefault()
    try {
      const r = await api.graph.query(slug, query)
      setQueryResult((r as any).result)
    } catch (err) {
      setQueryResult(String(err))
    }
  }

  return (
    <div>
      <h1>Knowledge Graph</h1>
      <div style={{ display: 'flex', gap: '0.5rem', marginBottom: '1rem', alignItems: 'center' }}>
        <input placeholder="project-slug" value={slug} onChange={e => setSlug(e.target.value)} />
        <button onClick={buildGraph} disabled={building}>{building ? 'Building…' : 'Build graph'}</button>
        {msg && <span style={{ fontSize: 13, color: msg.startsWith('Error') ? 'red' : 'green' }}>{msg}</span>}
      </div>
      <p style={{ color: '#666', fontSize: 13 }}>
        After building, open <code>~/.monolith/projects/{'<slug>'}/graph/graph.html</code> for the interactive visualisation.
      </p>
      <form onSubmit={runQuery} style={{ display: 'flex', gap: '0.5rem', marginBottom: '1rem' }}>
        <input
          placeholder="Query the graph…"
          value={query}
          onChange={e => setQuery(e.target.value)}
          style={{ flex: 1 }}
          required
        />
        <button type="submit">Query</button>
      </form>
      {queryResult && (
        <pre style={{ background: '#f9f9f9', padding: '0.75rem', borderRadius: 4, fontSize: 13, whiteSpace: 'pre-wrap' }}>
          {queryResult}
        </pre>
      )}
    </div>
  )
}
