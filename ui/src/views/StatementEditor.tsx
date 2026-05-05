import { useState } from 'react'
import { api, type Statement } from '../api'

export default function StatementEditor() {
  const [slug, setSlug] = useState('')
  const [statements, setStatements] = useState<Statement[]>([])
  const [content, setContent] = useState('')
  const [evidenceInput, setEvidenceInput] = useState('')
  const [error, setError] = useState('')
  const [provenance, setProvenance] = useState<object | null>(null)

  async function loadStatements() {
    const list = await api.statements.list(slug)
    setStatements(list)
  }

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault()
    setError('')
    const ids = evidenceInput.split(',').map(s => s.trim()).filter(Boolean)
    try {
      const stmt = await api.statements.create({ project_slug: slug, content, evidence_ids: ids })
      setStatements(prev => [stmt as Statement, ...prev])
      setContent(''); setEvidenceInput('')
    } catch (err) {
      setError(String(err))
    }
  }

  async function showProvenance(id: string) {
    const chain = await api.statements.provenance(id)
    setProvenance(chain)
  }

  return (
    <div>
      <h1>Statements</h1>
      <div style={{ display: 'flex', gap: '0.5rem', marginBottom: '1rem' }}>
        <input placeholder="project-slug" value={slug} onChange={e => setSlug(e.target.value)} />
        <button onClick={loadStatements}>Load</button>
      </div>
      <form onSubmit={handleCreate} style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem', maxWidth: 600, marginBottom: '1.5rem' }}>
        <textarea
          placeholder="Claim — what do you believe?"
          value={content}
          onChange={e => setContent(e.target.value)}
          rows={3}
          required
          style={{ resize: 'vertical' }}
        />
        <input
          placeholder="Evidence IDs (comma-separated)"
          value={evidenceInput}
          onChange={e => setEvidenceInput(e.target.value)}
          required
        />
        <button type="submit" style={{ alignSelf: 'flex-start' }}>Assert statement</button>
        {error && <span style={{ color: 'red' }}>{error}</span>}
      </form>
      {statements.map(s => (
        <div key={s.id} style={{ borderBottom: '1px solid #eee', padding: '0.5rem 0' }}>
          <p style={{ margin: '0 0 0.25rem' }}>{s.content}</p>
          <small style={{ color: '#888' }}>
            {s.evidence_ids.length} evidence · {s.created_at}
          </small>
          <button style={{ marginLeft: '1rem', fontSize: 12 }} onClick={() => showProvenance(s.id)}>Provenance</button>
        </div>
      ))}
      {provenance && (
        <div style={{ marginTop: '1.5rem', background: '#f9f9f9', padding: '0.75rem', borderRadius: 4 }}>
          <strong>Provenance chain</strong>
          <pre style={{ fontSize: 12, overflowX: 'auto' }}>{JSON.stringify(provenance, null, 2)}</pre>
        </div>
      )}
    </div>
  )
}
