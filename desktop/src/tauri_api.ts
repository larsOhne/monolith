/**
 * Typed wrappers around Tauri IPC commands and the dialog plugin.
 */
import { invoke } from '@tauri-apps/api/core'
import { open as dialogOpen } from '@tauri-apps/plugin-dialog'

// ── Types ─────────────────────────────────────────────────────────────────────

export interface VaultRecord {
  name: string
  path: string
}

export interface DirItem {
  name: string
  path: string
  is_dir: boolean
}

export interface VaultEntry {
  path: string
  name: string
  is_dir: boolean
  children: VaultEntry[]
}

// ── Vault registry ────────────────────────────────────────────────────────────

export const listVaults = () => invoke<VaultRecord[]>('list_vaults')

export const addVault = (path: string) => invoke<void>('add_vault', { path })

export const removeVault = (path: string) => invoke<void>('remove_vault', { path })

// ── File system ───────────────────────────────────────────────────────────────

export const browseDirectory = (path: string) =>
  invoke<DirItem[]>('browse_directory', { path })

export const readVaultFile = (vaultRoot: string, path: string) =>
  invoke<string>('read_vault_file', { vaultRoot, path })

export const writeVaultFile = (vaultRoot: string, path: string, content: string) =>
  invoke<void>('write_vault_file', { vaultRoot, path, content })

export const vaultTree = (vaultRoot: string) =>
  invoke<VaultEntry[]>('vault_tree', { vaultRoot })

export const createNote = (vaultRoot: string, parent: string, name: string) =>
  invoke<string>('create_note', { vaultRoot, parent, name })

export const deleteEntry = (vaultRoot: string, path: string) =>
  invoke<void>('delete_entry', { vaultRoot, path })

export const renameEntry = (vaultRoot: string, path: string, newName: string) =>
  invoke<string>('rename_entry', { vaultRoot, path, newName })

// ── Backend worker ────────────────────────────────────────────────────────────

export const startBackend = () => invoke<number>('start_backend')

export const getBackendPort = () => invoke<number | null>('get_backend_port')

export const stopBackend = () => invoke<void>('stop_backend')

// ── Native folder picker ──────────────────────────────────────────────────────

/** Opens the OS-native folder picker. Returns the selected path, or null. */
export async function pickFolder(): Promise<string | null> {
  const result = await dialogOpen({
    directory: true,
    multiple: false,
    title: 'Select Vault Folder',
  })
  if (!result) return null
  return typeof result === 'string' ? result : result[0] ?? null
}

/** Opens the OS-native file picker. Returns the selected path, or null. */
export async function pickFile(filters?: Array<{ name: string; extensions: string[] }>): Promise<string | null> {
  const result = await dialogOpen({
    directory: false,
    multiple: false,
    title: 'Select File',
    filters,
  })
  if (!result) return null
  return typeof result === 'string' ? result : result[0] ?? null
}
