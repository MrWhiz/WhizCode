import React, { useEffect, useState } from 'react'
import ReactDOM from 'react-dom'
import { 
  FiAlertCircle, 
  FiCheckCircle, 
  FiInfo, 
  FiX, 
  FiAlertTriangle 
} from 'react-icons/fi'

/**
 * Types of popups
 */
export type PopupType = 'info' | 'success' | 'warning' | 'error' | 'confirm'

interface PopupProps {
  isOpen: boolean
  onClose: () => void
  onConfirm?: () => void
  title: string
  message: string
  type?: PopupType
  confirmText?: string
  cancelText?: string
}

const Popup: React.FC<PopupProps> = ({
  isOpen,
  onClose,
  onConfirm,
  title,
  message,
  type = 'info',
  confirmText = 'Confirm',
  cancelText = 'Cancel'
}) => {
  const [mounted, setMounted] = useState(false)

  useEffect(() => {
    setMounted(true)
    return () => setMounted(false)
  }, [])

  if (!mounted || !isOpen) return null

  const getIcon = () => {
    switch (type) {
      case 'success': return <FiCheckCircle className="popup-icon success" />
      case 'warning': return <FiAlertTriangle className="popup-icon warning" />
      case 'error': return <FiAlertCircle className="popup-icon error" />
      case 'confirm': return <FiAlertTriangle className="popup-icon confirm" />
      default: return <FiInfo className="popup-icon info" />
    }
  }

  const handleBackdropClick = (e: React.MouseEvent) => {
    if (e.target === e.currentTarget) {
      onClose()
    }
  }

  return ReactDOM.createPortal(
    <div className="popup-overlay" onClick={handleBackdropClick}>
      <div className="popup-container glass-dark animate-fade-in">
        <div className="popup-header">
          <div className="popup-title-group">
            {getIcon()}
            <h3>{title}</h3>
          </div>
          <button className="popup-close" onClick={onClose}>
            <FiX />
          </button>
        </div>
        
        <div className="popup-content">
          <p>{message}</p>
        </div>
        
        <div className="popup-footer">
          {type === 'confirm' ? (
            <>
              <button className="btn-secondary" onClick={onClose}>
                {cancelText}
              </button>
              <button className="btn-primary danger" onClick={onConfirm}>
                {confirmText}
              </button>
            </>
          ) : (
            <button className="btn-primary" onClick={onClose}>
              Dismiss
            </button>
          )}
        </div>
      </div>
      
      <style>{`
        .popup-overlay {
          position: fixed;
          top: 0;
          left: 0;
          right: 0;
          bottom: 0;
          background: rgba(0, 0, 0, 0.4);
          backdrop-filter: blur(4px);
          display: flex;
          align-items: center;
          justify-content: center;
          z-index: 9999;
          padding: 20px;
        }

        .popup-container {
          width: 100%;
          max-width: 440px;
          border-radius: var(--radius-xl);
          box-shadow: var(--shadow-premium);
          padding: 24px;
          position: relative;
          overflow: hidden;
          border: 1px solid var(--border-color);
        }

        .popup-header {
          display: flex;
          align-items: center;
          justify-content: space-between;
          margin-bottom: 20px;
        }

        .popup-title-group {
          display: flex;
          align-items: center;
          gap: 12px;
        }

        .popup-title-group h3 {
          font-size: 1.1rem;
          font-weight: 600;
          color: var(--text-primary);
        }

        .popup-icon {
          font-size: 1.4rem;
        }

        .popup-icon.info { color: var(--accent-primary); }
        .popup-icon.success { color: var(--accent-vibrant); }
        .popup-icon.warning { color: #f59e0b; }
        .popup-icon.error { color: #ef4444; }
        .popup-icon.confirm { color: #8b5cf6; }

        .popup-close {
          background: transparent;
          border: none;
          color: var(--text-tertiary);
          font-size: 1.2rem;
          cursor: pointer;
          padding: 4px;
          border-radius: var(--radius-sm);
          display: flex;
          align-items: center;
          justify-content: center;
          transition: var(--transition-smooth);
        }

        .popup-close:hover {
          background: rgba(255, 255, 255, 0.05);
          color: var(--text-primary);
        }

        .popup-content {
          margin-bottom: 28px;
          color: var(--text-secondary);
          line-height: 1.6;
          font-size: 0.95rem;
        }

        .popup-footer {
          display: flex;
          align-items: center;
          justify-content: flex-end;
          gap: 12px;
        }

        .btn-primary, .btn-secondary {
          padding: 8px 20px;
          border-radius: var(--radius-md);
          font-size: 0.9rem;
          font-weight: 500;
          cursor: pointer;
          transition: var(--transition-smooth);
          border: none;
        }

        .btn-primary {
          background: var(--accent-primary);
          color: white;
        }

        .btn-primary:hover {
          filter: brightness(1.1);
          transform: translateY(-1px);
        }

        .btn-primary.danger {
          background: #ef4444;
        }

        .btn-secondary {
          background: rgba(255, 255, 255, 0.05);
          color: var(--text-primary);
          border: 1px solid var(--border-color);
        }

        .btn-secondary:hover {
          background: rgba(255, 255, 255, 0.1);
        }
      `}</style>
    </div>,
    document.body
  )
}

export default Popup
