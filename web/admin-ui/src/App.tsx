import { BrowserRouter } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 5 * 60 * 1000,
      retry: 1,
    },
  },
});

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <div style={{ padding: 24 }}>
          <h1>徽章管理系统</h1>
          <p>系统初始化中...</p>
        </div>
      </BrowserRouter>
    </QueryClientProvider>
  );
}

export default App;
