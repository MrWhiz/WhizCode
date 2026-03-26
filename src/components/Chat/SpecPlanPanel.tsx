import React from 'react'
import type { ExecutionPlanSnapshot, TaskSnapshot } from '../../lib/tauri-api'

interface SpecPlanPanelProps {
  currentPlan: ExecutionPlanSnapshot | null
  activeSpec: any | null
  taskSnapshot: TaskSnapshot | null
  isLoading: boolean
}

const statusColor = (status?: string) => {
  switch (status) {
    case 'completed':
      return '#86efac'
    case 'in_progress':
      return '#7dd3fc'
    case 'failed':
      return '#fda4af'
    case 'skipped':
      return '#cbd5e1'
    default:
      return '#fcd34d'
  }
}

export const SpecPlanPanel = ({
  currentPlan,
  activeSpec,
  taskSnapshot,
  isLoading,
}: SpecPlanPanelProps) => {
  const [isExpanded, setIsExpanded] = React.useState(false)
  const plan = activeSpec?.plan ?? currentPlan
  const acceptanceCriteria = plan?.acceptance_criteria ?? []
  const phases = taskSnapshot?.phases ?? []
  const totalTasks = phases.reduce((count, phase) => count + phase.tasks.length, 0)
  const completedTasks = phases.reduce(
    (count, phase) => count + phase.tasks.filter((task) => task.status === 'completed').length,
    0
  )

  if (!plan && !taskSnapshot) {
    return null
  }

  return (
    <div
      style={{
        margin: '8px 12px 0',
        padding: '10px 12px',
        borderRadius: '12px',
        border: '1px solid rgba(125, 211, 252, 0.18)',
        background: 'linear-gradient(180deg, rgba(20, 29, 48, 0.95), rgba(10, 16, 28, 0.92))',
        display: 'flex',
        flexDirection: 'column',
        gap: '8px',
        flexShrink: 0,
      }}
    >
      <div style={{ display: 'flex', justifyContent: 'space-between', gap: '8px', alignItems: 'center' }}>
        <div style={{ minWidth: 0 }}>
          <div style={{ fontSize: '10px', textTransform: 'uppercase', letterSpacing: '0.08em', color: '#7dd3fc', fontWeight: 800 }}>
            Spec Driven Plan
          </div>
          <div
            style={{
              fontSize: '12px',
              fontWeight: 700,
              color: 'var(--text-primary)',
              marginTop: '4px',
              whiteSpace: 'nowrap',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
            }}
            title={plan?.objective || taskSnapshot?.original_query || 'Current task'}
          >
            {plan?.objective || taskSnapshot?.original_query || 'Current task'}
          </div>
        </div>

        <div style={{ display: 'flex', gap: '6px', alignItems: 'center', flexShrink: 0 }}>
          <div
            style={{
              fontSize: '10px',
              fontWeight: 700,
              padding: '4px 8px',
              borderRadius: '999px',
              color: isLoading ? '#7dd3fc' : '#86efac',
              background: isLoading ? 'rgba(125, 211, 252, 0.12)' : 'rgba(134, 239, 172, 0.12)',
              border: `1px solid ${isLoading ? 'rgba(125, 211, 252, 0.24)' : 'rgba(134, 239, 172, 0.24)'}`,
            }}
          >
            {isLoading ? 'LIVE' : 'READY'}
          </div>
          <button
            type="button"
            onClick={() => setIsExpanded((value) => !value)}
            style={{
              background: 'rgba(255,255,255,0.04)',
              border: '1px solid rgba(255,255,255,0.08)',
              color: 'var(--text-secondary)',
              borderRadius: '8px',
              padding: '4px 8px',
              fontSize: '10px',
              fontWeight: 700,
              cursor: 'pointer',
            }}
          >
            {isExpanded ? 'Hide' : 'Show'}
          </button>
        </div>
      </div>

      <div
        style={{
          display: 'flex',
          gap: '8px',
          flexWrap: 'wrap',
          alignItems: 'center',
          fontSize: '11px',
          color: 'var(--text-secondary)',
        }}
      >
        <span>{completedTasks}/{totalTasks || 0} tasks done</span>
        {plan?.risk_level && <span>risk: {plan.risk_level}</span>}
        {phases.length > 0 && <span>{phases.length} phases</span>}
      </div>

      {isExpanded && (
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: '10px',
            maxHeight: '260px',
            overflowY: 'auto',
            paddingRight: '2px',
          }}
        >
          {plan?.spec_summary && (
            <div style={{ fontSize: '12px', lineHeight: 1.45, color: 'var(--text-secondary)' }}>
              {plan.spec_summary}
            </div>
          )}

          {acceptanceCriteria.length > 0 && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
              <div style={{ fontSize: '10px', textTransform: 'uppercase', color: 'var(--text-secondary)', fontWeight: 700 }}>
                Acceptance Criteria
              </div>
              {acceptanceCriteria.slice(0, 3).map((criterion: string, index: number) => (
                <div key={`${criterion}-${index}`} style={{ fontSize: '11px', color: 'var(--text-primary)', lineHeight: 1.4 }}>
                  {index + 1}. {criterion}
                </div>
              ))}
            </div>
          )}

          {phases.length > 0 && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
              <div style={{ fontSize: '10px', textTransform: 'uppercase', color: 'var(--text-secondary)', fontWeight: 700 }}>
                Live Task Board
              </div>
              {phases.map((phase) => (
                <div
                  key={phase.name}
                  style={{
                    border: '1px solid rgba(255,255,255,0.06)',
                    borderRadius: '10px',
                    padding: '10px',
                    background: 'rgba(255,255,255,0.02)',
                  }}
                >
                  <div style={{ fontSize: '11px', fontWeight: 700, color: 'var(--text-primary)', marginBottom: '6px' }}>
                    {phase.name}
                  </div>
                  <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                    {phase.tasks.map((task) => (
                      <div
                        key={task.id}
                        style={{
                          display: 'grid',
                          gridTemplateColumns: '1fr auto',
                          gap: '6px 10px',
                          alignItems: 'start',
                        }}
                      >
                        <div>
                          <div style={{ fontSize: '12px', color: 'var(--text-primary)', lineHeight: 1.35 }}>
                            {task.description}
                          </div>
                          <div style={{ fontSize: '10px', color: 'var(--text-secondary)', marginTop: '2px' }}>
                            {task.owner_agent || 'unassigned'}
                            {task.task_type ? ` · ${task.task_type}` : ''}
                            {task.requires_write ? ' · write' : ' · delegated'}
                          </div>
                        </div>
                        <div
                          style={{
                            fontSize: '10px',
                            fontWeight: 800,
                            color: statusColor(task.status),
                            background: `${statusColor(task.status)}18`,
                            border: `1px solid ${statusColor(task.status)}33`,
                            borderRadius: '999px',
                            padding: '3px 8px',
                            textTransform: 'uppercase',
                          }}
                        >
                          {task.status.replace('_', ' ')}
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  )
}
