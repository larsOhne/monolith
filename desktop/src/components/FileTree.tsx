import { useState } from 'react'
import { type VaultEntry } from '../tauri_api'
import { createNote, deleteEntry, renameEntry } from '../tauri_api'

interface Props {
  tree: VaultEntry[]
  vaultRoot: string
  openFilePath: string | null
  onOpenFile: (path: string) => void
  onRefresh: () => void
}

export default function FileTree({ tree, vaultRoot, openFilePath, onOpenFile, onRefresh }: Props) {
  return (
    <div style={{ overflowY: 'auto', height: '100%', padding: '6px 0' }}>
      {tree.map(e => (
        <TreeNode
          key={e.path}
          entry={e}
          depth={0}
          vaultRoot={vaultRoot}
          openFilePath={openFilePath}
          onOpenFile={onOpenFile}
          onRefresh={onRefresh}
        />
      ))}
    </div>
  )
}

interface NodeProps {
  entry: VaultEntry
  depth: number
  vaultRoot: string
  openFilePath: string | null
  onOpenFile: (path: string) => void
  onRefresh: () => void
}

function TreeNode({ entry, depth, vaultRoot, openFilePath, onOpenFile, onRefresh }: NodeProps) {
  const [expanded, setExpanded] = useState(depth < 1)
  const [ctxMenu, setCtxMenu] = useState<{ x: number; y: number } | null>(null)
  const [renaming, setRenaming] = useState(false)
  const [renameVal, setRenameVal] = useState(entry.name)
  const [creatingChild, setCreatingChild] = useState(false)
  const [newName, setNewName] = useState('')
  const isOpen = entry.path === openFilePath

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault()
    setCtxMenu({ x: e.clientX, y: e.clientY })
  }

  const closeCtx = () => setCtxMenu(null)

  const handleDelete = async () => {
    closeCtx()
    if (!confirm(`Delete "${entry.name}"?`)) return
    await deleteEntry(vaultRoot, entry.path)
    onRefresh()
  }

  const handleRename = async (e: React.FormEvent) => {
    e.preventDefault()
    await renameEntry(vaultRoot, entry.path, renameVal)
    setRenaming(false)
    onRefresh()
  }

  const handleNewNote = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!newName.trim()) return
    await createNote(vaultRoot, entry.path, newName.trim())
    setCreatingChild(false)
    setNewName('')
    setExpanded(true)
    onRefresh()
  }

  const indent = depth * 14 + 10

  if (renaming) {
    return (
      <form onSubmit={handleRename} style={{ paddingLeft: indent }}>
        <input
          autoFocus
          value={renameVal}
          onChange={e => setRenameVal(e.target.value)}
          onBlur={() => setRenaming(false)}
          onKeyDown={e => e.key === 'Escape' && setRenaming(false)}
          style={{ width: '90%', fontSize: 12, padding: '2px 6px' }}
        />
      </form>
    )
  }

  return (
    <>
      <div
        onContextMenu={handleContextMenu}
        onClick={() => entry.is_dir ? setExpanded(e => !e) : onOpenFile(entry.path)}
        style={{
          paddingLeft: indent,
          paddingRight: 8,
          paddingTop: 3,
          paddingBottom: 3,
          cursor: 'pointer',
          fontSize: 13,
          display: 'flex',
          alignItems: 'center',
          gap: 5,
          background: isOpen ? 'var(--surface2)' : 'transparent',
          borderLeft: isOpen ? '2px solid var(--accent)' : '2px solid transparent',
          color: entry.is_dir ? 'var(--fg)' : 'var(--muted)',
          userSelect: 'none',
        }}
      >
        <span style={{ width: 14, fontSize: 11, color: 'var(--muted)' }}>
          {entry.is_dir ? (expanded ? '▾' : '▸') : ''}
        </span>
        <span>{entry.is_dir ? '📁' : '📄'}</span>
        <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
          {entry.name}
        </span>
      </div>

      {entry.is_dir && expanded && (
        <>
          {entry.children.map(c => (
            <TreeNode
              key={c.path}
              entry={c}
              depth={depth + 1}
              vaultRoot={vaultRoot}
              openFilePath={openFilePath}
              onOpenFile={onOpenFile}
              onRefresh={onRefresh}
            />
          ))}
          {creatingChild && (
            <form onSubmit={handleNewNote} style={{ paddingLeft: indent + 28 }}>
              <input
                autoFocus
                placeholder="note-name"
                value={newName}
                onChange={e => setNewName(e.target.value)}
                onBlur={() => setCreatingChild(false)}
                onKeyDown={e => e.key === 'Escape' && setCreatingChild(false)}
                style={{ width: '85%', fontSize: 12, padding: '2px 6px' }}
              />
            </form>
          )}
        </>
      )}

      {/* Context menu */}
      {ctxMenu && (
        <>
          <div
            style={{ position: 'fixed', inset: 0, zIndex: 900 }}
            onClick={closeCtx}
          />
          <div style={{
            position: 'fixed',
            left: ctxMenu.x,
            top: ctxMenu.y,
            background: 'var(--surface)',
            border: '1px solid var(--border)',
            borderRadius: 'var(--radius)',
            zIndex: 1000,
            minWidth: 150,
            padding: '4px 0',
            boxShadow: '0 8px 24px rgba(0,0,0,0.4)',
          }}>
            {entry.is_dir && (
              <CtxItem
                label="New note here…"
                onClick={() => { closeCtx(); setCreatingChild(true); setExpanded(true) }}
              />
            )}
            <CtxItem label="Rename" onClick={() => { closeCtx(); setRenaming(true) }} />
            <CtxItem label="Delete" danger onClick={handleDelete} />
          </div>
        </>
      )}
    </>
  )
}

function CtxItem({ label, onClick, danger }: { label: string; onClick: () => void; danger?: boolean }) {
  return (
    <div
      onClick={onClick}
      style={{
        padding: '6px 14px',
        cursor: 'pointer',
        fontSize: 13,
        color: danger ? 'var(--danger)' : 'var(--fg)',
      }}
      onMouseEnter={e => (e.currentTarget.style.background = 'var(--surface2)')}
      onMouseLeave={e => (e.currentTarget.style.background = 'transparent')}
    >
      {label}
    </div>
  )
}
