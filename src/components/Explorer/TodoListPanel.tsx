import { useState, useEffect } from 'react';
import { 
  Box, 
  Typography, 
  TextField, 
  List, 
  ListItem, 
  ListItemText, 
  ListItemSecondaryAction, 
  IconButton, 
  Checkbox, 
  Divider,
  Fade,
  Grow
} from '@mui/material';

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
    <Box sx={{ p: 2, height: '100%', display: 'flex', flexDirection: 'column', color: 'var(--text-primary)' }}>
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 2 }}>
        <Typography variant="subtitle2" sx={{ fontWeight: 600, opacity: 0.8, letterSpacing: '0.5px' }}>
          TASKS & TO-DO
        </Typography>
        {todos.some(t => t.completed) && (
          <Typography 
            variant="caption" 
            onClick={clearCompleted}
            sx={{ 
              cursor: 'pointer', 
              opacity: 0.5, 
              '&:hover': { opacity: 0.8, color: 'rgb(239, 68, 68)' },
              transition: 'all 0.2s'
            }}
          >
            Clear Completed
          </Typography>
        )}
      </Box>
      
      <Box sx={{ display: 'flex', gap: 1, mb: 2 }}>
        <TextField
          fullWidth
          size="small"
          placeholder="Add a task..."
          value={inputValue}
          onChange={(e) => setInputValue(e.target.value)}
          onKeyDown={handleKeyPress}
          sx={{
            '& .MuiOutlinedInput-root': {
              color: 'inherit',
              backgroundColor: 'rgba(255, 255, 255, 0.05)',
              paddingRight: '8px',
              '& fieldset': { borderColor: 'rgba(255, 255, 255, 0.1)' },
              '&:hover fieldset': { borderColor: 'rgba(255, 255, 255, 0.2)' },
              '&.Mui-focused fieldset': { borderColor: 'var(--vscode-focusBorder, #007acc)' },
            },
            '& input::placeholder': { color: 'rgba(255, 255, 255, 0.3)' }
          }}
        />
        <IconButton 
          onClick={handleAddTodo} 
          disabled={!inputValue.trim()}
          sx={{ color: 'var(--vscode-button-background, #007acc)', p: '8px' }}
        >
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <line x1="12" y1="5" x2="12" y2="19"></line>
            <line x1="5" y1="12" x2="19" y2="12"></line>
          </svg>
        </IconButton>
      </Box>

      {/* Filter Chips */}
      <Box sx={{ display: 'flex', gap: 1, mb: 2, overflowX: 'auto', pb: 0.5 }}>
        {['all', 'active', 'completed'].map((f) => (
          <Box
            key={f}
            onClick={() => setFilter(f as any)}
            sx={{
              px: 1.5,
              py: 0.25,
              borderRadius: '12px',
              fontSize: '11px',
              cursor: 'pointer',
              backgroundColor: filter === f ? 'rgba(0, 122, 204, 0.2)' : 'rgba(255, 255, 255, 0.05)',
              border: '1px solid',
              borderColor: filter === f ? '#007acc' : 'transparent',
              color: filter === f ? '#72b1f1' : 'rgba(255, 255, 255, 0.5)',
              textTransform: 'capitalize',
              '&:hover': { backgroundColor: filter === f ? 'rgba(0, 122, 204, 0.3)' : 'rgba(255, 255, 255, 0.1)' }
            }}
          >
            {f}
          </Box>
        ))}
      </Box>

      <Box sx={{ flex: 1, overflowY: 'auto' }}>
        {filteredTodos.length === 0 ? (
          <Fade in timeout={800}>
            <Box sx={{ mt: 4, textAlign: 'center', opacity: 0.4 }}>
              <Typography variant="body2">
                {filter === 'all' ? 'No tasks yet.' : filter === 'active' ? 'No active tasks.' : 'No completed tasks.'}
              </Typography>
              {filter === 'all' && <Typography variant="caption">Start by adding one above.</Typography>}
            </Box>
          </Fade>
        ) : (
          <List sx={{ pt: 0 }}>
            {filteredTodos.map((todo: Todo, index: number) => (
              <Grow in key={todo.id} timeout={300 + index * 50}>
                <Box>
                  <ListItem
                    dense
                    sx={{
                      px: 1,
                      mb: 0.5,
                      borderRadius: '4px',
                      '&:hover': { backgroundColor: 'rgba(255, 255, 255, 0.03)' }
                    }}
                  >
                    <Checkbox
                      edge="start"
                      checked={todo.completed}
                      onChange={() => toggleTodo(todo.id)}
                      size="small"
                      sx={{ 
                        color: 'rgba(255, 255, 255, 0.2)',
                        '&.Mui-checked': { color: 'var(--vscode-button-background, #007acc)' }
                      }}
                    />
                    <ListItemText
                      primary={todo.text}
                      sx={{
                        '& .MuiTypography-root': {
                          fontSize: '13.5px',
                          textDecoration: todo.completed ? 'line-through' : 'none',
                          opacity: todo.completed ? 0.4 : 0.9,
                          whiteSpace: 'nowrap',
                          overflow: 'hidden',
                          textOverflow: 'ellipsis'
                        }
                      }}
                    />
                    <ListItemSecondaryAction>
                      <IconButton 
                        edge="end" 
                        size="small" 
                        onClick={() => deleteTodo(todo.id)}
                        sx={{ 
                          color: 'rgba(239, 68, 68, 0.3)',
                          '&:hover': { color: 'rgb(239, 68, 68)', backgroundColor: 'rgba(239, 68, 68, 0.1)' }
                        }}
                      >
                         <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                            <polyline points="3 6 5 6 21 6"></polyline>
                            <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
                         </svg>
                      </IconButton>
                    </ListItemSecondaryAction>
                  </ListItem>
                  <Divider sx={{ opacity: 0.03 }} />
                </Box>
              </Grow>
            ))}
          </List>
        )}
      </Box>
      
      <Box sx={{ mt: 'auto', pt: 2, borderTop: '1px solid rgba(255, 255, 255, 0.05)', opacity: 0.5 }}>
        <Typography variant="caption" sx={{ display: 'block', textAlign: 'center' }}>
          {todos.filter((t: Todo) => !t.completed).length} tasks remaining
        </Typography>
      </Box>
    </Box>
  );
};
