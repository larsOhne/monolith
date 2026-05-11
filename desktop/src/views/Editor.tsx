import { useState, useEffect, useCallback, useRef } from 'react'
import { readVaultFile, writeVaultFile, type VaultEntry } from '../tauri_api'
import type { ActiveVault } from '../App'

interface Props {
  vault: ActiveVault
  openFilePath: string | null
  onOpenFile: (path: string) => void
  onRefresh: () => void
}

/** Resolve [[wikilink]] → file path within the tree. */
function resolveWikilink(link: string, tree: VaultEntry[]): string | null {
  const target = link.toLowerCase().replace(/\.md$/, '')
  const flat: VaultEntry[] = []
  const flatten = (entries: VaultEntry[]) => {
    for (const e of entries) {
      if (!e.is_dir) flat.push(e)
      else flatten(e.children)
    }
  }
  flatten(tree)
  return flat.find(e => e.name.toLowerCase().replace(/\.md$/, '') === target)?.path ?? null
}

/** Very light markdown → HTML renderer. */
function renderMarkdown(md: string): string {
  const lines = md.split('\n')
  const html: string[] = []
  let inCode = false

  for (let line of lines) {
    if (line.startsWith('```')) {
      inCode = !inCode
      html.push(inCode ? '<pre><code>' : '</code></pre>')
      continue
    }
    if (inCode) { html.push(escHtml(line)); continue }

    // Headings
    const hm = line.match(/^(#{1,6}) (.+)/)
    if (hm) { html.push(`<h${hm[1].length}>${inline(hm[2])}</h${hm[1].length}>`); continue }

    // HR
    if (/^---+$/.test(line.trim())) { html.push('<hr>'); continue }

    // List item
    const lm = line.match(/^(\s*)[-*+] (.+)/)
    if (lm) { html.push(`<li style="margin-left:${lm[1].length * 12}px">${inline(lm[2])}</li>`); continue }

    // Blank line
    if (!line.trim()) { html.push('<br>'); continue }

    html.push(`<p>${inline(line)}</p>`)
  }
  return html.join('\n')
}

function escHtml(s: string) {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
}

function inline(s: string): string {
  return s
    .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
    .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
    .replace(/\*(.+?)\*/g, '<em>$1</em>')
    .replace(/`(.+?)`/g, '<code>$1</code>')
    .replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2">$1</a>')
    .replace(/\[\[([^\]]+)\]\]/g, '<span style="color:var(--accent);cursor:pointer" data-wikilink="$1">$1</span>')
}

export default function Editor({ vault, openFilePath, onOpenFile, onRefresh }: Props) {
  const [content, setContent] = useState('')
  const [dirty, setDirty] = useState(false)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [preview, setPreview] = useState(false)
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null)

  // Load file when openFilePath changes
  useEffect(() => {
    if (!openFilePath) { setContent(''); setDirty(false); return }
    readVaultFile(vault.record.path, openFilePath)
      .then(c => { setContent(c); setDirty(false); setError(null) })
      .catch(e => setError(String(e)))
  }, [openFilePath, vault.record.path])

  // Autosave after 1 s idle
  const scheduleAutosave = useCallback((text: string) => {
    if (saveTimer.current) clearTimeout(saveTimer.current)
    saveTimer.current = setTimeout(async () => {
      if (!openFilePath) return
      setSaving(true)
      try {
        await writeVaultFile(vault.record.path, openFilePath, text)
        setDirty(false)
      } catch (e) {
        setError(String(e))
      } finally {
        setSaving(false)
      }
    }, 1000)
  }, [openFilePath, vault.record.path])

  const handleChange = (text: string) => {
    setContent(text)
    setDirty(true)
    scheduleAutosave(text)
  }

  // Click wikilinks in preview
  const handlePreviewClick = (e: React.MouseEvent<HTMLDivElement>) => {
    const target = e.target as HTMLElement
    const link = target.dataset.wikilink
    if (link) {
      const resolved = resolveWikilink(link, vault.tree)
      if (resolved) onOpenFile(resolved)
    }
  }

  const filename = openFilePath ? openFilePath.split('/').pop() : null

  if (!openFilePath) {
    return (
      <div style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--muted)', flexDirection: 'column', gap: 8 }}>
        <span style={{ fontSize: 36 }}>📝</span>
        <span>Select a file to edit</span>
      </div>
    )
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      {/* Toolbar */}
      <div style={{
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        padding: '6px 14px',
        borderBottom: '1px solid var(--border)',
        background: 'var(--surface)',
        flexShrink: 0,
      }}>
        <span style={{ flex: 1, fontWeight: 500, fontSize: 13, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', fontFamily: 'var(--font-mono)' }}>
          {filename}
          {dirty && <span style={{ color: 'var(--muted)', marginLeft: 6 }}>●</span>}
        </span>
        {saving && <span style={{ color: 'var(--muted)', fontSize: 11 }}>Saving…</span>}
        {error && <span style={{ color: 'var(--danger)', fontSize: 11 }}>{error}</span>}
        <button
          onClick={() => setPreview(p => !p)}
          style={{ padding: '3px 10px', fontSize: 12, background: preview ? 'var(--accent)' : undefined }}
        >
          {preview ? 'Edit' : 'Preview'}
        </button>
      </div>

      {/* Editor / Preview */}
      <div style={{ flex: 1, overflow: 'hidden', display: 'flex' }}>
        {preview ? (
          <div
            onClick={handlePreviewClick}
            style={{
              flex: 1,
              padding: '24px 32px',
              overflowY: 'auto',
              lineHeight: 1.7,
              maxWidth: 820,
              margin: '0 auto',
              width: '100%',
            }}
            dangerouslySetInnerHTML={{ __html: renderMarkdown(content) }}
          />
        ) : (
          <textarea
            value={content}
            onChange={e => handleChange(e.target.value)}
            spellCheck={false}
            style={{
              flex: 1,
              background: 'var(--bg)',
              color: 'var(--fg)',
              border: 'none',
              outline: 'none',
              padding: '24px 32px',
              fontFamily: 'var(--font-mono)',
              fontSize: 14,
              lineHeight: 1.7,
              resize: 'none',
              width: '100%',
            }}
          />
        )}
      </div>
    </div>
  )
}
