import { useState, useEffect } from 'react'
import { listVaults, addVault, removeVault, pickFolder, type VaultRecord } from '../tauri_api'

interface Props {
  onClose: () => void
  onOpenVault: (record: VaultRecord) => void
}

export default function VaultManager({ onClose, onOpenVault }: Props) {
  const [vaults, setVaults] = useState<VaultRecord[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const reload = async () => {
    setVaults(await listVaults())
  }

  useEffect(() => { reload() }, [])

  const handleAdd = async () => {
    setError(null)
    const path = await pickFolder()
    if (!path) return
    setLoading(true)
    try {
      await addVault(path)
      await reload()
    } catch (e) {
      setError(String(e))
    } finally {
      setLoading(false)
    }
  }

  const handleRemove = async (path: string) => {
    await removeVault(path)
    await reload()
  }

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={e => e.stopPropagation()} style={{ minWidth: 520 }}>
        <h2>Vault Manager</h2>

        {vaults.length === 0 ? (
          <p style={{ color: 'var(--muted)', fontSize: 13 }}>No vaults registered yet.</p>
        ) : (
          <div style={{
            border: '1px solid var(--border)',
            borderRadius: 'var(--radius)',
            overflow: 'hidden',
            maxHeight: 360,
            overflowY: 'auto',
          }}>
            {vaults.map(v => (
              <div
                key={v.path}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  padding: '10px 14px',
                  borderBottom: '1px solid var(--border)',
                  gap: 12,
                }}
              >
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ fontWeight: 500, fontSize: 13 }}>{v.name}</div>
                  <div style={{ fontSize: 11, color: 'var(--muted)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{v.path}</div>
                </div>
                <button onClick={() => onOpenVault(v)} className="primary" style={{ padding: '3px 12px', fontSize: 12 }}>Open</button>
                <button onClick={() => handleRemove(v.path)} className="danger" style={{ padding: '3px 10px', fontSize: 12 }}>✕</button>
              </div>
            ))}
          </div>
        )}

        {error && <div style={{ color: 'var(--danger)', fontSize: 12 }}>{error}</div>}

        <div className="modal-footer">
          <button onClick={handleAdd} disabled={loading}>
            {loading ? 'Opening…' : '+ Add Vault…'}
          </button>
          <button onClick={onClose}>Close</button>
        </div>
      </div>
    </div>
  )
}
