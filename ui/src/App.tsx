import { Routes, Route, NavLink } from 'react-router-dom'
import ProjectsView from './views/ProjectsView'
import SourceViewer from './views/SourceViewer'
import StatementEditor from './views/StatementEditor'
import GraphView from './views/GraphView'
import DriftInspector from './views/DriftInspector'

export default function App() {
  return (
    <div style={{ display: 'flex', height: '100vh', fontFamily: 'system-ui, sans-serif' }}>
      <nav style={{ width: 200, padding: '1rem', borderRight: '1px solid #ddd', display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
        <strong style={{ marginBottom: '0.5rem' }}>Monolith</strong>
        <NavLink to="/">Projects</NavLink>
        <NavLink to="/sources">Sources</NavLink>
        <NavLink to="/statements">Statements</NavLink>
        <NavLink to="/graph">Graph</NavLink>
        <NavLink to="/drift">Drift</NavLink>
      </nav>
      <main style={{ flex: 1, overflow: 'auto', padding: '1.5rem' }}>
        <Routes>
          <Route path="/" element={<ProjectsView />} />
          <Route path="/sources" element={<SourceViewer />} />
          <Route path="/statements" element={<StatementEditor />} />
          <Route path="/graph" element={<GraphView />} />
          <Route path="/drift" element={<DriftInspector />} />
        </Routes>
      </main>
    </div>
  )
}
