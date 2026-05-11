import { useState, useEffect, useCallback } from 'react'
import VaultManager from './components/VaultManager'
import FileTree from './components/FileTree'
import QuickSwitcher from './components/QuickSwitcher'
import Editor from './views/Editor'
import SourcesView from './views/SourcesView'
import GraphView from './views/GraphView'
import DriftInspector from './views/DriftInspector'
import { vaultTree, startBackend, type VaultEntry, type VaultRecord } from './tauri_api'

type View = 'editor' | 'sources' | 'graph' | 'drift'

export interface ActiveVault {
  record: VaultRecord
  tree: VaultEntry[]
}

export default function App() {
  const [vault, setVault] = useState<ActiveVault | null>(null)
  const [view, setView] = useState<View>('editor')
  const [openFilePath, setOpenFilePath] = useState<string | null>(null)
  const [backendPort, setBackendPort] = useState<number | null>(null)
  const [vaultManagerOpen, setVaultManagerOpen] = useState(false)
  const [quickSwitcherOpen, setQuickSwitcherOpen] = useState(false)
  const [backendStatus, setBackendStatus] = useState<'idle' | 'starting' | 'ready' | 'error'>('idle')
  const [backendError, setBackendError] = useState<string | null>(null)

  // ── Open a vault ──────────────────────────────────────────────────────────
  const openVault = useCallback(async (record: VaultRecord) => {
    const tree = await vaultTree(record.path)
    setVault({ record, tree })
    setOpenFilePath(null)
    setVaultManagerOpen(false)
  }, [])

  // ── Refresh file tree ─────────────────────────────────────────────────────
  const refreshTree = useCallback(async () => {
    if (!vault) return
    const tree = await vaultTree(vault.record.path)
    setVault(v => v ? { ...v, tree } : null)
  }, [vault])

  // ── Start Python backend ──────────────────────────────────────────────────
  useEffect(() => {
    setBackendStatus('starting')
    startBackend()
      .then(port => {
        setBackendPort(port)
        setBackendStatus('ready')
      })
      .catch(err => {
        setBackendError(String(err))
        setBackendStatus('error')
      })
  }, [])

  // ── Global keyboard shortcuts ─────────────────────────────────────────────
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
        e.preventDefault()
        if (vault) setQuickSwitcherOpen(o => !o)
      }
      if (e.key === 'Escape') {
        setQuickSwitcherOpen(false)
        setVaultManagerOpen(false)
      }
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [vault])

  // ── Derive base URL for the Python API ────────────────────────────────────
  const apiBase = backendPort ? `http://127.0.0.1:${backendPort}` : null

  return (
    <div style={{ display: 'flex', height: '100vh', overflow: 'hidden' }}>
      {/* ── Sidebar ──────────────────────────────────────────────────────── */}
      <aside style={{
        width: 220,
        background: 'var(--surface)',
        borderRight: '1px solid var(--border)',
        display: 'flex',
        flexDirection: 'column',
        flexShrink: 0,
      }}>
        {/* Vault header */}
        <div style={{ padding: '12px 14px', borderBottom: '1px solid var(--border)' }}>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 6 }}>
            <span style={{ fontSize: 13, fontWeight: 600, color: 'var(--fg)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
              {vault?.record.name ?? 'No vault open'}
            </span>
            <button
              onClick={() => setVaultManagerOpen(true)}
              style={{ padding: '2px 8px', fontSize: 11, flexShrink: 0, marginLeft: 6 }}
              title="Manage vaults"
            >
              ⌂
            </button>
          </div>
          {vault && (
            <button
              onClick={() => setQuickSwitcherOpen(true)}
              style={{ width: '100%', textAlign: 'left', padding: '4px 8px', fontSize: 12, color: 'var(--muted)' }}
            >
              🔍 Quick open  <kbd style={{ float: 'right', fontSize: 10, color: 'var(--muted)' }}>Ctrl+K</kbd>
            </button>
          )}
        </div>

        {/* Navigation */}
        <nav style={{ padding: '8px 8px 4px', borderBottom: '1px solid var(--border)' }}>
          {(['editor', 'sources', 'graph', 'drift'] as View[]).map(v => (
            <button
              key={v}
              onClick={() => setView(v)}
              style={{
                display: 'block',
                width: '100%',
                textAlign: 'left',
                marginBottom: 2,
                background: view === v ? 'var(--accent)' : 'transparent',
                border: 'none',
                color: view === v ? '#fff' : 'var(--muted)',
                padding: '5px 10px',
                borderRadius: 'var(--radius)',
                fontSize: 13,
              }}
            >
              {{ editor: '✏️  Editor', sources: '📄  Sources', graph: '🕸  Graph', drift: '⚡  Drift' }[v]}
            </button>
          ))}
        </nav>

        {/* File tree (editor mode only) */}
        {view === 'editor' && vault && (
          <div style={{ flex: 1, overflow: 'hidden' }}>
            <FileTree
              tree={vault.tree}
              vaultRoot={vault.record.path}
              openFilePath={openFilePath}
              onOpenFile={setOpenFilePath}
              onRefresh={refreshTree}
            />
          </div>
        )}

        {/* Backend status */}
        <div style={{
          padding: '8px 14px',
          borderTop: '1px solid var(--border)',
          fontSize: 11,
          color: backendStatus === 'ready' ? 'var(--green)' : backendStatus === 'error' ? 'var(--danger)' : 'var(--muted)',
        }}>
          {backendStatus === 'starting' && '⏳ Starting backend…'}
          {backendStatus === 'ready' && `✓ Backend :${backendPort}`}
          {backendStatus === 'error' && `✗ Backend error`}
          {backendStatus === 'error' && backendError && (
            <div title={backendError} style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
              {backendError}
            </div>
          )}
        </div>
      </aside>

      {/* ── Main area ────────────────────────────────────────────────────── */}
      <main style={{ flex: 1, overflow: 'hidden', display: 'flex', flexDirection: 'column' }}>
        {!vault && view === 'editor' ? (
          <div style={{
            flex: 1,
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            gap: 16,
            color: 'var(--muted)',
          }}>
            <div style={{ fontSize: 48 }}>🗂</div>
            <div style={{ fontSize: 18, color: 'var(--fg)' }}>No vault open</div>
            <div style={{ fontSize: 13 }}>Open or create a vault to get started.</div>
            <button className="primary" onClick={() => setVaultManagerOpen(true)}>
              Open Vault…
            </button>
          </div>
        ) : (
          <>
            {view === 'editor' && (
              <Editor
                vault={vault!}
                openFilePath={openFilePath}
                onOpenFile={setOpenFilePath}
                onRefresh={refreshTree}
              />
            )}
            {view === 'sources'  && <SourcesView apiBase={apiBase} />}
            {view === 'graph'    && <GraphView   apiBase={apiBase} />}
            {view === 'drift'    && <DriftInspector apiBase={apiBase} />}
          </>
        )}
      </main>

      {/* ── Modals ───────────────────────────────────────────────────────── */}
      {vaultManagerOpen && (
        <VaultManager
          onClose={() => setVaultManagerOpen(false)}
          onOpenVault={openVault}
        />
      )}
      {quickSwitcherOpen && vault && (
        <QuickSwitcher
          tree={vault.tree}
          onSelect={path => { setOpenFilePath(path); setQuickSwitcherOpen(false) }}
          onClose={() => setQuickSwitcherOpen(false)}
        />
      )}
    </div>
  )
}
