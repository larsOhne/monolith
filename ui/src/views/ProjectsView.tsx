import { useEffect, useState } from 'react'
import { api, type Project } from '../api'

export default function ProjectsView() {
  const [projects, setProjects] = useState<Project[]>([])
  const [name, setName] = useState('')
  const [slug, setSlug] = useState('')
  const [desc, setDesc] = useState('')
  const [error, setError] = useState('')

  useEffect(() => { api.projects.list().then(setProjects).catch(console.error) }, [])

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault()
    setError('')
    try {
      const p = await api.projects.create({ name, slug, description: desc })
      setProjects(prev => [...prev, p as Project])
      setName(''); setSlug(''); setDesc('')
    } catch (err) {
      setError(String(err))
    }
  }

  return (
    <div>
      <h1>Projects</h1>
      <form onSubmit={handleCreate} style={{ display: 'flex', gap: '0.5rem', marginBottom: '1.5rem', flexWrap: 'wrap' }}>
        <input placeholder="Name" value={name} onChange={e => setName(e.target.value)} required />
        <input placeholder="slug" value={slug} onChange={e => setSlug(e.target.value)} required />
        <input placeholder="Description (optional)" value={desc} onChange={e => setDesc(e.target.value)} />
        <button type="submit">Create</button>
        {error && <span style={{ color: 'red' }}>{error}</span>}
      </form>
      <table style={{ width: '100%', borderCollapse: 'collapse' }}>
        <thead>
          <tr>{['Name', 'Slug', 'Sources', 'Evidence', 'Issues'].map(h => (
            <th key={h} style={{ textAlign: 'left', padding: '0.4rem', borderBottom: '1px solid #ddd' }}>{h}</th>
          ))}</tr>
        </thead>
        <tbody>
          {(projects as any[]).map((p) => (
            <tr key={p.id}>
              <td style={{ padding: '0.4rem' }}>{p.name}</td>
              <td style={{ padding: '0.4rem' }}><code>{p.slug}</code></td>
              <td style={{ padding: '0.4rem' }}>{p.source_count ?? '—'}</td>
              <td style={{ padding: '0.4rem' }}>{p.evidence_count ?? '—'}</td>
              <td style={{ padding: '0.4rem', color: p.evidence_with_issues > 0 ? 'orange' : 'green' }}>
                {p.evidence_with_issues ?? 0}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}
