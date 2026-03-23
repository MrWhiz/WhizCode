import { useEffect } from 'react'
import type { Message, AgentStep } from '../types'

export function useAutoScroll(
  messagesEndRef: React.RefObject<HTMLDivElement | null>,
  messages: Message[],
  agentSteps: AgentStep[]
) {
  useEffect(() => {
    if (messagesEndRef.current && messagesEndRef.current.parentElement) {
      const parent = messagesEndRef.current.parentElement
      const isNearBottom = parent.scrollHeight - parent.scrollTop - parent.clientHeight < 150
      if (isNearBottom) {
        messagesEndRef.current.scrollIntoView({ behavior: 'smooth' })
      }
    }
  }, [messages, agentSteps, messagesEndRef])
}
