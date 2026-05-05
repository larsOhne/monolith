import { useState } from 'react'
import { api, type Source, type Evidence } from '../api'

export default function SourceViewer() {
  const [slug, setSlug] = useState('')
  const [sources, setSources] = useState<Source[]>([])
  const [selected, setSelected] = useState<Source | null>(null)
  const [content, setContent] = useState('')
  const [evidence, setEvidence] = useState<Evidence[]>([])
  const [selection, setSelection] = useState('')
  const [pinMsg, setPinMsg] = useState('')

  async function loadSources() {
    const list = await api.sources.list(slug)
    setSources(list)
    setSelected(null); setContent(''); setEvidence([])
  }

  async function openSource(s: Source) {
    setSelected(s)
    const { content: text } = await api.sources.content(s.id)
    setContent(text)
    setPinMsg('')
  }

  async function pinSelection() {
    if (!selected || !selection) return
    try {
      await api.evidence.pin({ source_id: selected.id, verbatim_text: selection })
      setPinMsg('Pinned!')
    } catch (err) {
      setPinMsg(String(err))
    }
  }

  return (
    <div>
      <h1>Source Viewer</h1>
      <div style={{ display: 'flex', gap: '0.5rem', marginBottom: '1rem' }}>
        <input placeholder="project-slug" value={slug} onChange={e => setSlug(e.target.value)} />
        <button onClick={loadSources}>Load sources</button>
      </div>
      <div style={{ display: 'flex', gap: '1rem', height: 'calc(100vh - 200px)' }}>
        {/* Source list */}
        <aside style={{ width: 220, overflowY: 'auto', borderRight: '1px solid #ddd', paddingRight: '0.5rem' }}>
          {sources.map(s => (
            <div
              key={s.id}
              onClick={() => openSource(s)}
              style={{ cursor: 'pointer', padding: '0.3rem', background: selected?.id === s.id ? '#eef' : 'transparent', borderRadius: 4 }}
            >
              {s.path}
            </div>
          ))}
        </aside>
        {/* Content */}
        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
          {selected && (
            <>
              <p style={{ margin: 0, color: '#666', fontSize: 13 }}>
                Select text, then click Pin to create evidence.
                {pinMsg && <span style={{ marginLeft: 8, color: pinMsg.startsWith('Error') ? 'red' : 'green' }}>{pinMsg}</span>}
              </p>
              <button
                style={{ alignSelf: 'flex-start' }}
                onClick={() => { const sel = window.getSelection()?.toString() ?? ''; setSelection(sel); pinSelection() }}
              >
                Pin selection
              </button>
              <pre style={{ flex: 1, overflowY: 'auto', background: '#f9f9f9', padding: '0.75rem', borderRadius: 4, fontSize: 13, whiteSpace: 'pre-wrap', wordBreak: 'break-word' }}>
                {content}
              </pre>
            </>
          )}
        </div>
      </div>
    </div>
  )
}
