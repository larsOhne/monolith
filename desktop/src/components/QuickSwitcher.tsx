import { useState, useRef, useEffect } from 'react'
import { type VaultEntry } from '../tauri_api'

/** Flatten tree to searchable list of file paths */
function flattenFiles(entries: VaultEntry[], acc: VaultEntry[] = []): VaultEntry[] {
  for (const e of entries) {
    if (!e.is_dir) acc.push(e)
    else flattenFiles(e.children, acc)
  }
  return acc
}

interface Props {
  tree: VaultEntry[]
  onSelect: (path: string) => void
  onClose: () => void
}

export default function QuickSwitcher({ tree, onSelect, onClose }: Props) {
  const [query, setQuery] = useState('')
  const [cursor, setCursor] = useState(0)
  const inputRef = useRef<HTMLInputElement>(null)

  useEffect(() => { inputRef.current?.focus() }, [])

  const files = flattenFiles(tree)
  const q = query.toLowerCase()
  const results = q
    ? files.filter(f => f.name.toLowerCase().includes(q) || f.path.toLowerCase().includes(q))
    : files.slice(0, 20)

  useEffect(() => { setCursor(0) }, [query])

  const pick = (path: string) => { onSelect(path); onClose() }

  const handleKey = (e: React.KeyboardEvent) => {
    if (e.key === 'ArrowDown') { e.preventDefault(); setCursor(c => Math.min(c + 1, results.length - 1)) }
    if (e.key === 'ArrowUp')   { e.preventDefault(); setCursor(c => Math.max(c - 1, 0)) }
    if (e.key === 'Enter' && results[cursor]) pick(results[cursor].path)
    if (e.key === 'Escape') onClose()
  }

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div
        className="modal"
        onClick={e => e.stopPropagation()}
        style={{ minWidth: 500, padding: 0, gap: 0 }}
        onKeyDown={handleKey}
      >
        <div style={{ padding: '12px 16px', borderBottom: '1px solid var(--border)' }}>
          <input
            ref={inputRef}
            value={query}
            onChange={e => setQuery(e.target.value)}
            placeholder="Search files…"
            style={{ width: '100%', fontSize: 15, background: 'transparent', border: 'none', outline: 'none', color: 'var(--fg)' }}
          />
        </div>
        <div style={{ maxHeight: 360, overflowY: 'auto' }}>
          {results.length === 0 && (
            <div style={{ padding: '16px', color: 'var(--muted)', fontSize: 13 }}>No files found.</div>
          )}
          {results.map((f, i) => (
            <div
              key={f.path}
              onClick={() => pick(f.path)}
              style={{
                padding: '8px 16px',
                cursor: 'pointer',
                background: i === cursor ? 'var(--surface2)' : 'transparent',
                borderLeft: i === cursor ? '3px solid var(--accent)' : '3px solid transparent',
              }}
            >
              <div style={{ fontSize: 13, fontWeight: 500 }}>{f.name}</div>
              <div style={{ fontSize: 11, color: 'var(--muted)', fontFamily: 'var(--font-mono)', marginTop: 1 }}>
                {f.path}
              </div>
            </div>
          ))}
        </div>
        <div style={{ padding: '8px 16px', borderTop: '1px solid var(--border)', fontSize: 11, color: 'var(--muted)', display: 'flex', gap: 16 }}>
          <span><kbd>↑↓</kbd> navigate</span>
          <span><kbd>↵</kbd> open</span>
          <span><kbd>Esc</kbd> close</span>
        </div>
      </div>
    </div>
  )
}
