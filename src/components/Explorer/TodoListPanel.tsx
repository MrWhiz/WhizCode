import { useState, useEffect } from 'react';

interface Todo {
  id: string;
  text: string;
  completed: boolean;
  createdAt: number;
}

export const TodoListPanel = () => {
  const [todos, setTodos] = useState<Todo[]>(() => {
    const saved = localStorage.getItem('whizcode_todos');
    return saved ? JSON.parse(saved) : [];
  });
  const [inputValue, setInputValue] = useState('');
  const [filter, setFilter] = useState<'all' | 'active' | 'completed'>('all');

  useEffect(() => {
    localStorage.setItem('whizcode_todos', JSON.stringify(todos));
  }, [todos]);

  const handleAddTodo = () => {
    if (!inputValue.trim()) return;
    const newTodo: Todo = {
      id: Math.random().toString(36).substr(2, 9),
      text: inputValue.trim(),
      completed: false,
      createdAt: Date.now(),
    };
    setTodos([newTodo, ...todos]);
    setInputValue('');
  };

  const toggleTodo = (id: string) => {
    setTodos(todos.map((todo: Todo) => 
      todo.id === id ? { ...todo, completed: !todo.completed } : todo
    ));
  };

  const deleteTodo = (id: string) => {
    setTodos(todos.filter((todo: Todo) => todo.id !== id));
  };

  const clearCompleted = () => {
    setTodos(todos.filter((todo: Todo) => !todo.completed));
  };

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      handleAddTodo();
    }
  };

  const filteredTodos = todos.filter((todo: Todo) => {
    if (filter === 'active') return !todo.completed;
    if (filter === 'completed') return todo.completed;
    return true;
  });

  return (
    <div style={{ padding: '16px', height: '100%', display: 'flex', flexDirection: 'column', color: 'var(--text-primary)' }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '16px' }}>
        <div style={{ fontWeight: 600, opacity: 0.8, letterSpacing: '0.5px', fontSize: '12px' }}>
          TASKS & TO-DO
        </div>
        {todos.some(t => t.completed) && (
          <div 
            onClick={clearCompleted}
            style={{ 
              cursor: 'pointer', 
              opacity: 0.5, 
              fontSize: '12px',
              transition: 'all 0.2s'
            }}
            onMouseEnter={(e) => {
              (e.currentTarget as HTMLElement).style.opacity = '0.8';
              (e.currentTarget as HTMLElement).style.color = 'rgb(239, 68, 68)';
            }}
            onMouseLeave={(e) => {
              (e.currentTarget as HTMLElement).style.opacity = '0.5';
              (e.currentTarget as HTMLElement).style.color = 'inherit';
            }}
          >
            Clear Completed
          </div>
        )}
      </div>
      
      <div style={{ display: 'flex', gap: '8px', marginBottom: '16px' }}>
        <input
          type="text"
          placeholder="Add a task..."
          value={inputValue}
          onChange={(e) => setInputValue(e.target.value)}
          onKeyDown={handleKeyPress}
          style={{
            flex: 1,
            padding: '8px',
            backgroundColor: 'rgba(255, 255, 255, 0.05)',
            border: '1px solid rgba(255, 255, 255, 0.1)',
            borderRadius: '4px',
            color: 'inherit',
            fontSize: '13px'
          }}
        />
        <button 
          onClick={handleAddTodo} 
          disabled={!inputValue.trim()}
          style={{ 
            padding: '8px 12px',
            backgroundColor: 'var(--vscode-button-background, #007acc)',
            color: 'white',
            border: 'none',
            borderRadius: '4px',
            cursor: inputValue.trim() ? 'pointer' : 'not-allowed',
            opacity: inputValue.trim() ? 1 : 0.5
          }}
        >
          +
        </button>
      </div>

      {/* Filter Chips */}
      <div style={{ display: 'flex', gap: '8px', marginBottom: '16px', overflowX: 'auto', paddingBottom: '4px' }}>
        {['all', 'active', 'completed'].map((f) => (
          <div
            key={f}
            onClick={() => setFilter(f as any)}
            style={{
              padding: '4px 12px',
              borderRadius: '12px',
              fontSize: '11px',
              cursor: 'pointer',
              backgroundColor: filter === f ? 'rgba(0, 122, 204, 0.2)' : 'rgba(255, 255, 255, 0.05)',
              border: '1px solid ' + (filter === f ? '#007acc' : 'transparent'),
              color: filter === f ? '#72b1f1' : 'rgba(255, 255, 255, 0.5)',
              textTransform: 'capitalize',
              whiteSpace: 'nowrap'
            }}
            onMouseEnter={(e) => {
              (e.currentTarget as HTMLElement).style.backgroundColor = filter === f ? 'rgba(0, 122, 204, 0.3)' : 'rgba(255, 255, 255, 0.1)';
            }}
            onMouseLeave={(e) => {
              (e.currentTarget as HTMLElement).style.backgroundColor = filter === f ? 'rgba(0, 122, 204, 0.2)' : 'rgba(255, 255, 255, 0.05)';
            }}
          >
            {f}
          </div>
        ))}
      </div>

      <div style={{ flex: 1, overflowY: 'auto' }}>
        {filteredTodos.length === 0 ? (
          <div style={{ marginTop: '64px', textAlign: 'center', opacity: 0.4 }}>
            <div style={{ fontSize: '13px' }}>
              {filter === 'all' ? 'No tasks yet.' : filter === 'active' ? 'No active tasks.' : 'No completed tasks.'}
            </div>
            {filter === 'all' && <div style={{ fontSize: '12px' }}>Start by adding one above.</div>}
          </div>
        ) : (
          <div style={{ paddingTop: 0 }}>
            {filteredTodos.map((todo: Todo) => (
              <div key={todo.id}>
                <div
                  style={{
                    padding: '8px',
                    marginBottom: '4px',
                    borderRadius: '4px',
                    display: 'flex',
                    alignItems: 'center',
                    gap: '8px'
                  }}
                  onMouseEnter={(e) => {
                    (e.currentTarget as HTMLElement).style.backgroundColor = 'rgba(255, 255, 255, 0.03)';
                  }}
                  onMouseLeave={(e) => {
                    (e.currentTarget as HTMLElement).style.backgroundColor = 'transparent';
                  }}
                >
                  <input
                    type="checkbox"
                    checked={todo.completed}
                    onChange={() => toggleTodo(todo.id)}
                    style={{ 
                      cursor: 'pointer',
                      accentColor: 'var(--vscode-button-background, #007acc)'
                    }}
                  />
                  <span
                    style={{
                      fontSize: '13.5px',
                      textDecoration: todo.completed ? 'line-through' : 'none',
                      opacity: todo.completed ? 0.4 : 0.9,
                      whiteSpace: 'nowrap',
                      overflow: 'hidden',
                      textOverflow: 'ellipsis',
                      flex: 1
                    }}
                  >
                    {todo.text}
                  </span>
                  <button 
                    onClick={() => deleteTodo(todo.id)}
                    style={{ 
                      background: 'none',
                      border: 'none',
                      color: 'rgba(239, 68, 68, 0.3)',
                      cursor: 'pointer',
                      padding: '4px',
                      fontSize: '14px'
                    }}
                    onMouseEnter={(e) => {
                      (e.currentTarget as HTMLElement).style.color = 'rgb(239, 68, 68)';
                      (e.currentTarget as HTMLElement).style.backgroundColor = 'rgba(239, 68, 68, 0.1)';
                    }}
                    onMouseLeave={(e) => {
                      (e.currentTarget as HTMLElement).style.color = 'rgba(239, 68, 68, 0.3)';
                      (e.currentTarget as HTMLElement).style.backgroundColor = 'transparent';
                    }}
                  >
                    ✕
                  </button>
                </div>
                <div style={{ borderBottom: '1px solid rgba(255, 255, 255, 0.03)' }} />
              </div>
            ))}
          </div>
        )}
      </div>
      
      <div style={{ marginTop: 'auto', paddingTop: '16px', borderTop: '1px solid rgba(255, 255, 255, 0.05)', opacity: 0.5, textAlign: 'center', fontSize: '12px' }}>
        {todos.filter((t: Todo) => !t.completed).length} tasks remaining
      </div>
    </div>
  );
};
